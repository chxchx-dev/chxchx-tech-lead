#pragma once

#include <QObject>
#include <QProcess>
#include <QTimer>

#include <memory>

class QTemporaryFile;

struct ImageDescriptionResult {
    QString imagePath;
    QString markdown;
    QString error;
    bool ok() const { return error.isEmpty() && !markdown.trimmed().isEmpty(); }
};

// A project-local Markdown cache and an explicit, read-only vision request.
class AgentChatImageDescriptions final : public QObject {
    Q_OBJECT
public:
    AgentChatImageDescriptions(QString cliProgram, QString projectPath, QString agentId, QObject *parent = nullptr);
    ~AgentChatImageDescriptions() override;

    static QString cachedDescription(const QString &projectPath, const QString &imagePath);
    static QString cachePath(const QString &projectPath, const QString &imagePath);
    static bool store(const QString &projectPath, const QString &imagePath, const QString &markdown,
                      const QString &agentId, const QString &model, QString *error = nullptr);

    bool isBusy() const;
    QString currentImage() const { return m_imagePath; }
    void describe(const QString &imagePath, const QString &model = {}, const QString &effort = {});
    void cancel();

signals:
    void finished(const ImageDescriptionResult &result);
    void busyChanged(bool busy, const QString &imagePath);

private:
    void complete(int exitCode);
    void fail(QString message);
    static QString hashFile(const QString &path);

    QString m_cliProgram;
    QString m_projectPath;
    QString m_agentId;
    QString m_model;
    QString m_imagePath;
    QProcess m_process;
    QTimer m_timeout;
    QTimer m_killTimer;
    QByteArray m_output;
    QByteArray m_errorOutput;
    std::unique_ptr<QTemporaryFile> m_promptFile;
    bool m_cancelled = false;
};
