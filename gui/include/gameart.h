// SPDX-License-Identifier: LicenseRef-AGPL-3.0-only-OpenSSL

#ifndef CHIAKI_GAMEART_H
#define CHIAKI_GAMEART_H

#include <QColor>
#include <QNetworkAccessManager>
#include <QObject>
#include <QSet>
#include <QString>
#include <QStringList>

#include <functional>

// Cover art of the game that is open on the console, so the interface can take
// the look of that game. The PS5 tells its name through discovery; the cover is
// looked up on Wikipedia at run time and kept in the user's data folder (it is
// never shipped with the app).
class GameArt : public QObject
{
	Q_OBJECT

	Q_PROPERTY(bool enabled READ enabled WRITE setEnabled NOTIFY enabledChanged)
	Q_PROPERTY(QString gameName READ gameName NOTIFY artChanged)
	Q_PROPERTY(QString coverUrl READ coverUrl NOTIFY artChanged)
	Q_PROPERTY(QColor accent READ accent NOTIFY artChanged)
	Q_PROPERTY(bool active READ active NOTIFY artChanged)
	// sube cada vez que llega la portada de algún tema
	Q_PROPERTY(int coversRevision READ coversRevision NOTIFY coversChanged)

public:
	explicit GameArt(QObject *parent = nullptr);

	bool enabled() const { return is_enabled; }
	void setEnabled(bool enabled);
	QString gameName() const { return game_name; }
	QString coverUrl() const { return cover_url; }
	QColor accent() const { return accent_color; }
	bool active() const { return is_enabled && !cover_url.isEmpty(); }

	// name and title id as the console reports them; empty name = no game open
	void setRunningGame(const QString &name, const QString &title_id);

	// cover of any game by name (for the game themes); empty until it is downloaded
	int coversRevision() const { return covers_revision; }
	Q_INVOKABLE QString coverFor(const QString &game_name);

signals:
	void enabledChanged();
	void artChanged();
	void coversChanged();

private:
	QNetworkAccessManager network;
	bool is_enabled = true;
	QString game_name;
	QString requested_key;
	QString cover_url;
	QColor accent_color;
	int covers_revision = 0;
	QSet<QString> pending_covers;
	QStringList official_wallpapers; // names on playstation.com/wallpapers
	bool official_loaded = false;
	QList<std::function<void()>> official_waiting;

	QString cacheDir() const;
	QString cacheKey(const QString &name, const QString &title_id) const;
	QString cachedCover(const QString &key) const;
	void showCover(const QString &path);
	// looks the cover up and calls done(path) once it is saved (not called on failure)
	void fetchCover(const QString &name, const QString &key, std::function<void(const QString &)> done);
	void downloadCover(const QUrl &url, const QString &key, std::function<void(const QString &)> done);
	void withOfficialWallpapers(std::function<void()> then);
	QString officialWallpaperFor(const QString &name) const;
	void fetchThemeCover(const QString &name, const QString &key);
};

#endif // CHIAKI_GAMEART_H
