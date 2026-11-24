// SPDX-License-Identifier: LicenseRef-AGPL-3.0-only-OpenSSL

#include "gameart.h"

#include <QCryptographicHash>
#include <climits>
#include <QDir>
#include <QFileInfo>
#include <QFile>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QRegularExpression>
#include <QSettings>
#include <QStandardPaths>
#include <QUrl>
#include <QUrlQuery>
#include <QDebug>

static const char *USER_AGENT = "NitroPlay/1.0 (https://github.com/pilahito/NitroPlay)";
static const qint64 MAX_COVER_BYTES = 8 * 1024 * 1024;

GameArt::GameArt(QObject *parent) : QObject(parent)
{
	is_enabled = QSettings().value("theme/game_art", true).toBool();
}

void GameArt::setEnabled(bool enabled)
{
	if(is_enabled == enabled)
		return;
	is_enabled = enabled;
	QSettings().setValue("theme/game_art", enabled);
	emit enabledChanged();
	emit artChanged();
}

QString GameArt::cacheDir() const
{
	return QDir(QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation)).filePath("juegos");
}

// the title id is the best key; the name is the fallback (some consoles omit the id)
QString GameArt::cacheKey(const QString &name, const QString &title_id) const
{
	static const QRegularExpression valid_id("^[A-Z]{4}[0-9]{5}$");
	if(valid_id.match(title_id).hasMatch())
		return title_id;
	return QString::fromLatin1(QCryptographicHash::hash(name.toUtf8(), QCryptographicHash::Sha1).toHex().left(16));
}

void GameArt::setRunningGame(const QString &name, const QString &title_id)
{
	if(name.isEmpty())
	{
		if(game_name.isEmpty())
			return;
		game_name.clear();
		requested_key.clear();
		cover_url.clear();
		emit artChanged();
		return;
	}

	const QString key = cacheKey(name, title_id);
	if(key == requested_key)
		return;
	requested_key = key;
	game_name = name;
	cover_url.clear();
	emit artChanged();

	const QString cached = cachedCover(key);
	if(!cached.isEmpty())
	{
		showCover(cached);
		return;
	}
	fetchCover(name, key, [this, key](const QString &path) {
		if(key == requested_key)
			showCover(path);
	});
}

QString GameArt::cachedCover(const QString &key) const
{
	const QDir dir(cacheDir());
	const QStringList found = dir.entryList({ key + ".*", key + "-oficial.*" }, QDir::Files);
	return found.isEmpty() ? QString() : dir.filePath(found.first());
}

QString GameArt::coverFor(const QString &game_name)
{
	if(game_name.isEmpty())
		return {};
	const QString key = cacheKey(game_name, QString());
	const QString cached = cachedCover(key);
	if(!cached.isEmpty())
		return QUrl::fromLocalFile(cached).toString();
	if(!pending_covers.contains(key))
	{
		pending_covers.insert(key);
		fetchThemeCover(game_name, key);
	}
	return {};
}

// an official wallpaper from playstation.com when there is one, otherwise the cover
void GameArt::fetchThemeCover(const QString &name, const QString &key)
{
	auto finished = [this, key](const QString &) {
		pending_covers.remove(key);
		covers_revision++;
		emit coversChanged();
	};
	withOfficialWallpapers([this, name, key, finished]() {
		const QString wallpaper = officialWallpaperFor(name);
		if(wallpaper.isEmpty())
		{
			fetchCover(name, key, finished);
			return;
		}
		QUrl url("https://gmedia.playstation.com/is/image/SIEPDC/" + wallpaper);
		url.setQuery("wid=2560");
		downloadCover(url, key + "-oficial", finished);
	});
}

// the list of official wallpapers is read once per run from playstation.com
void GameArt::withOfficialWallpapers(std::function<void()> then)
{
	if(official_loaded)
	{
		then();
		return;
	}
	official_waiting.append(then);
	if(official_waiting.size() > 1)
		return;
	QNetworkRequest request(QUrl("https://www.playstation.com/es-es/wallpapers/"));
	request.setHeader(QNetworkRequest::UserAgentHeader, USER_AGENT);
	QNetworkReply *reply = network.get(request);
	connect(reply, &QNetworkReply::finished, this, [this, reply]() {
		reply->deleteLater();
		if(reply->error() == QNetworkReply::NoError)
		{
			static const QRegularExpression re("gmedia\\.playstation\\.com/is/image/SIEPDC/([A-Za-z0-9_.%-]+)");
			const QString html = QString::fromUtf8(reply->read(4 * 1024 * 1024));
			QSet<QString> names;
			for(auto it = re.globalMatch(html); it.hasNext();)
				names.insert(it.next().captured(1));
			official_wallpapers = names.values();
			official_wallpapers.sort();
		}
		else
			qWarning() << "Game art: official wallpapers unavailable:" << reply->errorString();
		official_loaded = true;
		const auto waiting = official_waiting;
		official_waiting.clear();
		for(const auto &then : waiting)
			then();
	});
}

// a landscape wallpaper whose file name contains the game's name
QString GameArt::officialWallpaperFor(const QString &name) const
{
	QString slug = name.toLower();
	slug.remove(QRegularExpression("^marvel'?s "));
	slug.replace(QRegularExpression("[^a-z0-9]+"), "-");
	slug.remove(QRegularExpression("^-|-$"));
	if(slug.size() < 4)
		return {};
	static const QRegularExpression portrait("mobile|phone|galaxy|pixel|ipad|tablet|1080x1920|39x18|vertical|logo|favicon|spinner", QRegularExpression::CaseInsensitiveOption);
	static const QRegularExpression desktop("desktop|keyart|key-art", QRegularExpression::CaseInsensitiveOption);
	static const QRegularExpression large("3840|4k|2560", QRegularExpression::CaseInsensitiveOption);
	QString best;
	int best_score = -1;
	for(const auto &candidate : official_wallpapers)
	{
		if(!candidate.toLower().contains(slug) || portrait.match(candidate).hasMatch())
			continue;
		const int score = (desktop.match(candidate).hasMatch() ? 2 : 0) + (large.match(candidate).hasMatch() ? 1 : 0);
		if(score > best_score)
		{
			best = candidate;
			best_score = score;
		}
	}
	return best;
}

// the accent is the most saturated colour that is common in the cover
void GameArt::showCover(const QString &path)
{
	QImage image(path);
	if(image.isNull())
		return;
	const QImage small = image.scaled(48, 48, Qt::IgnoreAspectRatio, Qt::SmoothTransformation).convertToFormat(QImage::Format_RGB32);
	QHash<int, int> buckets;
	QHash<int, QColor> sample;
	for(int y = 0; y < small.height(); y++)
		for(int x = 0; x < small.width(); x++)
		{
			const QColor c = QColor::fromRgb(small.pixel(x, y));
			if(c.hsvSaturation() < 90 || c.value() < 70)
				continue;
			const int bucket = c.hsvHue() / 15;
			buckets[bucket] += 1 + c.hsvSaturation() / 64;
			sample[bucket] = c;
		}
	QColor accent("#53b5ff");
	int best = 0;
	for(auto it = buckets.cbegin(); it != buckets.cend(); ++it)
		if(it.value() > best) { best = it.value(); accent = sample.value(it.key()); }
	// bright enough to read on a dark background
	accent = QColor::fromHsv(accent.hsvHue(), qMin(255, qMax(accent.hsvSaturation(), 140)), qMax(accent.value(), 210));

	accent_color = accent;
	cover_url = QUrl::fromLocalFile(path).toString();
	emit artChanged();
}

void GameArt::fetchCover(const QString &name, const QString &key, std::function<void(const QString &)> done)
{
	QString clean = name;
	clean.remove(QRegularExpression("[™®©]"));
	QUrl url("https://en.wikipedia.org/w/api.php");
	QUrlQuery query;
	query.addQueryItem("action", "query");
	query.addQueryItem("generator", "search");
	query.addQueryItem("gsrsearch", clean.simplified() + " video game");
	query.addQueryItem("gsrlimit", "8");
	query.addQueryItem("prop", "pageimages");
	query.addQueryItem("piprop", "original");
	query.addQueryItem("pilicense", "any");
	query.addQueryItem("format", "json");
	query.addQueryItem("formatversion", "2");
	url.setQuery(query);

	QNetworkRequest request(url);
	request.setHeader(QNetworkRequest::UserAgentHeader, USER_AGENT);
	qInfo() << "Game art: looking up cover for" << clean;
	QNetworkReply *reply = network.get(request);
	const QString name_for_rank = clean.simplified();
	connect(reply, &QNetworkReply::finished, this, [this, reply, key, done, name_for_rank]() {
		reply->deleteLater();
		if(reply->error() != QNetworkReply::NoError)
		{
			qWarning() << "Game art: search failed:" << reply->errorString();
			return;
		}
		const auto pages = QJsonDocument::fromJson(reply->readAll()).object()
			.value("query").toObject().value("pages").toArray();
		// the best result has a picture, a title that starts with the game's name
		// and is not a manga, film or compilation
		static const QRegularExpression other_media("manga|anime|film|novel|soundtrack|collection|comic", QRegularExpression::CaseInsensitiveOption);
		QUrl image;
		int best_rank = INT_MAX;
		for(const auto &value : pages)
		{
			const QJsonObject page = value.toObject();
			const QString source = page.value("original").toObject().value("source").toString();
			const QString title = page.value("title").toString();
			if(source.isEmpty() || other_media.match(title).hasMatch())
				continue;
			const int rank = page.value("index").toInt() + (title.startsWith(name_for_rank, Qt::CaseInsensitive) ? 0 : 100);
			if(rank < best_rank)
			{
				best_rank = rank;
				image = QUrl(source);
			}
		}
		// only images served by Wikimedia itself
		if(image.scheme() == "https" && image.host() == "upload.wikimedia.org")
			downloadCover(image, key, done);
	});
}

void GameArt::downloadCover(const QUrl &url, const QString &key, std::function<void(const QString &)> done)
{
	QNetworkRequest request(url);
	request.setHeader(QNetworkRequest::UserAgentHeader, USER_AGENT);
	QNetworkReply *reply = network.get(request);
	connect(reply, &QNetworkReply::finished, this, [this, reply, key, url, done]() {
		reply->deleteLater();
		if(reply->error() != QNetworkReply::NoError)
		{
			qWarning() << "Game art: download failed:" << reply->errorString();
			return;
		}
		const QByteArray data = reply->read(MAX_COVER_BYTES + 1);
		if(data.size() > MAX_COVER_BYTES || QImage::fromData(data).isNull())
			return;
		QString suffix = QFileInfo(url.path()).suffix().toLower();
		if(suffix != "png" && suffix != "jpg" && suffix != "jpeg" && suffix != "webp")
			suffix = "jpg";
		QDir().mkpath(cacheDir());
		const QString path = QDir(cacheDir()).filePath(key + "." + suffix);
		QFile file(path);
		if(!file.open(QIODevice::WriteOnly) || file.write(data) != data.size())
			return;
		file.close();
		done(path);
	});
}
