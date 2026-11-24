// SPDX-License-Identifier: LicenseRef-AGPL-3.0-only-OpenSSL

#include "streamcapture.h"

#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QGuiApplication>
#include <QScreen>
#include <QStandardPaths>
#include <QUrl>
#include <QWindow>

#ifndef _WIN32
#include <signal.h>
#endif

StreamCapture::StreamCapture(QObject *parent) : QObject(parent)
{
#if defined(Q_OS_LINUX)
	program = QStandardPaths::findExecutable("wf-recorder");
#endif
	record_process.setProcessChannelMode(QProcess::MergedChannels);
	live_process.setProcessChannelMode(QProcess::MergedChannels);

	connect(&record_process, &QProcess::started, this, &StreamCapture::recordingChanged);
	connect(&live_process, &QProcess::started, this, &StreamCapture::liveChanged);
	connect(&record_process, &QProcess::finished, this, [this]() {
		processFinished(record_process, stopping_record, record_software, false);
	});
	connect(&live_process, &QProcess::finished, this, [this]() {
		processFinished(live_process, stopping_live, live_software, true);
	});
}

StreamCapture::~StreamCapture()
{
	stopAll();
	record_process.waitForFinished(3000);
	live_process.waitForFinished(3000);
}

QString StreamCapture::recordingsFolder() const
{
	QString base = QStandardPaths::writableLocation(QStandardPaths::MoviesLocation);
	if(base.isEmpty())
		base = QDir::homePath();
	return QDir(base).filePath("NitroPlay");
}

int StreamCapture::liveService() const
{
	return settings.value("capture/live_service", Twitch).toInt();
}

void StreamCapture::setLiveService(int service)
{
	settings.setValue("capture/live_service", service);
	emit liveSettingsChanged();
}

QString StreamCapture::liveKey() const
{
	return settings.value("capture/live_key").toString();
}

void StreamCapture::setLiveKey(const QString &key)
{
	settings.setValue("capture/live_key", key.trimmed());
	emit liveSettingsChanged();
}

QString StreamCapture::liveUrl() const
{
	return settings.value("capture/live_url").toString();
}

void StreamCapture::setLiveUrl(const QString &url)
{
	settings.setValue("capture/live_url", url.trimmed());
	emit liveSettingsChanged();
}

bool StreamCapture::liveReady() const
{
	if(liveService() == Custom)
		return liveUrl().startsWith("rtmp");
	return !liveKey().isEmpty();
}

QString StreamCapture::liveTarget() const
{
	switch(liveService())
	{
		case Twitch:
			return "rtmp://live.twitch.tv/app/" + liveKey();
		case YouTube:
			return "rtmp://a.rtmp.youtube.com/live2/" + liveKey();
		default:
		{
			QString url = liveUrl();
			if(!liveKey().isEmpty())
				url += (url.endsWith('/') ? "" : "/") + liveKey();
			return url;
		}
	}
}

void StreamCapture::setStatus(const QString &text)
{
	status_text = text;
	emit statusChanged();
}

// Screen the game window is on, and the sound that is playing on this PC.
QStringList StreamCapture::captureArgs(bool software) const
{
	QStringList args = { "-y" };

	QScreen *screen = nullptr;
	const auto windows = QGuiApplication::topLevelWindows();
	for(QWindow *w : windows)
		if(w->isVisible() && w->screen()) { screen = w->screen(); break; }
	if(!screen)
		screen = QGuiApplication::primaryScreen();
	if(screen && !screen->name().isEmpty())
		args << "-o" << screen->name();

	QProcess pactl;
	pactl.start("pactl", { "get-default-sink" });
	if(pactl.waitForFinished(2000))
	{
		const QString sink = QString::fromUtf8(pactl.readAllStandardOutput()).trimmed();
		if(!sink.isEmpty())
			args << QString("--audio=%1.monitor").arg(sink);
	}

	if(software)
		args << "-c" << "libx264" << "-x" << "yuv420p" << "-p" << "preset=veryfast";
	else
		args << "-c" << "h264_nvenc" << "-x" << "yuv420p" << "-p" << "preset=p5";
	args << "-r" << "60";
	return args;
}

void StreamCapture::startRecording()
{
	QDir().mkpath(recordingsFolder());
	last_record_file = QDir(recordingsFolder()).filePath(
		QString("NitroPlay-%1.mkv").arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm-ss")));
	QStringList args = captureArgs(record_software);
	args << "-p" << (record_software ? "crf=20" : "cq=21");
	args << "-f" << last_record_file;
	stopping_record = false;
	record_process.start(program, args);
	setStatus(tr("Grabando en %1").arg(QDir::toNativeSeparators(recordingsFolder())));
}

void StreamCapture::startLive()
{
	QStringList args = captureArgs(live_software);
	// what Twitch and YouTube ask for: 1080p, constant 6 Mbps, a keyframe every 2 s
	args << "-F" << "scale=1920:1080";
	if(!live_software)
		args << "-p" << "rc=cbr";
	args << "-p" << "b=6000k" << "-p" << "maxrate=6000k" << "-p" << "bufsize=12000k" << "-p" << "g=120";
	args << "-C" << "aac" << "-m" << "flv" << "-f" << liveTarget();
	stopping_live = false;
	live_process.start(program, args);
	setStatus(tr("En directo"));
}

void StreamCapture::stopProcess(QProcess &process, bool &stopping)
{
	if(process.state() == QProcess::NotRunning)
		return;
	stopping = true;
#ifndef _WIN32
	// SIGINT lets wf-recorder finish the file properly
	::kill(static_cast<pid_t>(process.processId()), SIGINT);
#else
	process.terminate();
#endif
}

void StreamCapture::processFinished(QProcess &process, bool &stopping, bool &software, bool is_live)
{
	const QString output = QString::fromUtf8(process.readAll());
	const bool user_stopped = stopping;
	stopping = false;
	if(is_live)
		emit liveChanged();
	else
		emit recordingChanged();

	if(user_stopped)
	{
		setStatus(is_live ? tr("Directo terminado") : tr("Vídeo guardado: %1").arg(QDir::toNativeSeparators(last_record_file)));
		return;
	}

	// the graphics card encoder could not start: try again without it
	const bool encoder_failed = output.contains("Unknown encoder") || output.contains("Cannot load")
		|| output.contains("No capable devices") || output.contains("OpenEncodeSession")
		|| output.contains("Could not open codec") || output.contains("Error while opening encoder");
	if(!software && encoder_failed)
	{
		software = true;
		if(is_live)
			startLive();
		else
			startRecording();
		return;
	}

	// never show the full output: for a live stream it contains the stream key
	QString reason = tr("se ha detenido");
	if(output.contains("Connection refused") || output.contains("Input/output error") || output.contains("Server returned"))
		reason = tr("el servicio rechazó la conexión (revisa la clave de retransmisión)");
	else if(output.contains("Failed to connect") || output.contains("No such output"))
		reason = tr("no se pudo capturar la pantalla");
	setStatus(is_live ? tr("El directo falló: %1").arg(reason) : tr("La grabación falló: %1").arg(reason));
}

void StreamCapture::toggleRecording()
{
	if(!available())
	{
		setStatus(tr("Falta wf-recorder: instálalo con «sudo pacman -S wf-recorder»"));
		return;
	}
	if(recording())
		stopProcess(record_process, stopping_record);
	else
		startRecording();
}

void StreamCapture::toggleLive()
{
	if(!available())
	{
		setStatus(tr("Falta wf-recorder: instálalo con «sudo pacman -S wf-recorder»"));
		return;
	}
	if(live())
	{
		stopProcess(live_process, stopping_live);
		return;
	}
	if(!liveReady())
	{
		setStatus(tr("Pon tu clave de retransmisión en Ajustes → General → Directo"));
		return;
	}
	startLive();
}

void StreamCapture::openRecordingsFolder()
{
	QDir().mkpath(recordingsFolder());
	QDesktopServices::openUrl(QUrl::fromLocalFile(recordingsFolder()));
}

void StreamCapture::stopAll()
{
	stopProcess(record_process, stopping_record);
	stopProcess(live_process, stopping_live);
}
