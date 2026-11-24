// SPDX-License-Identifier: LicenseRef-AGPL-3.0-only-OpenSSL

#ifndef CHIAKI_STREAMCAPTURE_H
#define CHIAKI_STREAMCAPTURE_H

#include <QObject>
#include <QProcess>
#include <QSettings>
#include <QStringList>

// Records the game to a video file or sends it live to Twitch, YouTube or any
// RTMP server, using wf-recorder (Wayland screen capture) with the graphics
// card encoder when available. Recording and live streaming are independent.
class StreamCapture : public QObject
{
	Q_OBJECT

	Q_PROPERTY(bool available READ available CONSTANT)
	Q_PROPERTY(bool recording READ recording NOTIFY recordingChanged)
	Q_PROPERTY(bool live READ live NOTIFY liveChanged)
	Q_PROPERTY(QString status READ status NOTIFY statusChanged)
	Q_PROPERTY(QString recordingsFolder READ recordingsFolder CONSTANT)
	Q_PROPERTY(int liveService READ liveService WRITE setLiveService NOTIFY liveSettingsChanged)
	Q_PROPERTY(QString liveKey READ liveKey WRITE setLiveKey NOTIFY liveSettingsChanged)
	Q_PROPERTY(QString liveUrl READ liveUrl WRITE setLiveUrl NOTIFY liveSettingsChanged)
	Q_PROPERTY(bool liveReady READ liveReady NOTIFY liveSettingsChanged)

public:
	enum Service { Twitch = 0, YouTube = 1, Custom = 2 };
	Q_ENUM(Service)

	explicit StreamCapture(QObject *parent = nullptr);
	~StreamCapture();

	bool available() const { return !program.isEmpty(); }
	bool recording() const { return record_process.state() != QProcess::NotRunning; }
	bool live() const { return live_process.state() != QProcess::NotRunning; }
	QString status() const { return status_text; }
	QString recordingsFolder() const;

	int liveService() const;
	void setLiveService(int service);
	QString liveKey() const;
	void setLiveKey(const QString &key);
	QString liveUrl() const;
	void setLiveUrl(const QString &url);
	bool liveReady() const;

	Q_INVOKABLE void toggleRecording();
	Q_INVOKABLE void toggleLive();
	Q_INVOKABLE void openRecordingsFolder();
	Q_INVOKABLE void stopAll();

signals:
	void recordingChanged();
	void liveChanged();
	void statusChanged();
	void liveSettingsChanged();

private:
	QString program;
	QSettings settings;
	QProcess record_process;
	QProcess live_process;
	QString status_text;
	QString last_record_file;
	bool record_software = false;
	bool live_software = false;
	bool stopping_record = false;
	bool stopping_live = false;

	void setStatus(const QString &text);
	QStringList captureArgs(bool software) const;
	QString liveTarget() const;
	void startRecording();
	void startLive();
	void stopProcess(QProcess &process, bool &stopping);
	void processFinished(QProcess &process, bool &stopping, bool &software, bool is_live);
};

#endif // CHIAKI_STREAMCAPTURE_H
