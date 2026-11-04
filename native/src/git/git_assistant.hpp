#pragma once

#include <QObject>
#include <QProcess>
#include <QTimer>

#include <memory>

class QTemporaryFile;

struct GitAssistantResult {
    QString text;
    QString error;
    bool cancelled = false;
    bool ok() const { return error.isEmpty() && !cancelled && !text.trimmed().isEmpty(); }
};

// Asks one of the project's agents a single question through `chxchx-tech agent chat --lean`: the CLI
// keeps the authority over trust and sessions, and the turn is read-only and leaves no session behind.
// The prompt travels in a temporary file (a diff would not fit a command line) that is removed afterwards.
class GitAssistant final : public QObject {
    Q_OBJECT
public:
    GitAssistant(QString cliProgram, QString projectPath, QObject *parent = nullptr);
    ~GitAssistant() override;

    void setCliProgram(const QString &program) { m_cli = program; }
    bool isBusy() const { return m_process.state() != QProcess::NotRunning; }
    // `model` and `effort` are optional: empty means the agent's own configuration.
    void ask(const QString &agentId, const QString &prompt, const QString &model = QString(), const QString &effort = QString());
    void cancel();
    // The assistant's text and error from the agent-event lines the CLI printed.
    static GitAssistantResult parseOutput(const QByteArray &standardOutput, int exitCode, const QString &standardError);

signals:
    void finished(const GitAssistantResult &result);

private:
    void finish(int exitCode);
    void fail(const QString &message);

    QString m_cli;
    QString m_projectPath;
    QProcess m_process;
    std::unique_ptr<QTemporaryFile> m_promptFile;
    QTimer m_timeout;
    QByteArray m_output;
    bool m_cancelled = false;
    bool m_timedOut = false;
};
