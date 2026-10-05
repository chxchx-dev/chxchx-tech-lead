#pragma once

#include <QObject>
#include <QProcess>
#include <QString>
#include <QStringList>

class BridgeClient final : public QObject {
    Q_OBJECT

public:
    explicit BridgeClient(QString workingDirectory, QObject *parent = nullptr);

    bool isRunning() const;
    void setWorkingDirectory(const QString &path);
    bool execute(const QStringList &arguments);
    QByteArray readStandardOutput();
    QByteArray readStandardError();

signals:
    void finished(int exitCode, QProcess::ExitStatus status);
    void failedToStart(const QString &reason);

private:
    QString m_program;
    QProcess m_process;
};
