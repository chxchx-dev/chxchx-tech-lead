#include "git_assistant.hpp"

#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryFile>

namespace {

constexpr int TimeoutMs = 4 * 60 * 1000;
constexpr int KillAfterMs = 4000;

} // namespace

GitAssistant::GitAssistant(QString cliProgram, QString projectPath, QObject *parent)
    : QObject(parent), m_cli(std::move(cliProgram)), m_projectPath(std::move(projectPath))
{
    m_timeout.setSingleShot(true);
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    connect(&m_process, &QProcess::readyReadStandardOutput, this, [this] { m_output += m_process.readAllStandardOutput(); });
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this, [this](int code, QProcess::ExitStatus) { finish(code); });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) fail(QStringLiteral("No pude iniciar chxchx-tech: %1").arg(m_process.errorString()));
    });
    connect(&m_timeout, &QTimer::timeout, this, [this] {
        m_timedOut = true;
        cancel();
    });
}

GitAssistant::~GitAssistant()
{
    if (m_process.state() != QProcess::NotRunning) {
        m_process.disconnect(this);
        m_process.kill();
        m_process.waitForFinished(1000);
    }
}

void GitAssistant::fail(const QString &message)
{
    GitAssistantResult result;
    result.error = message;
    m_promptFile.reset();
    m_timeout.stop();
    emit finished(result);
}

void GitAssistant::ask(const QString &agentId, const QString &prompt, const QString &model, const QString &effort)
{
    if (isBusy()) return;
    if (m_cli.isEmpty()) {
        QTimer::singleShot(0, this, [this] { fail(QStringLiteral("No se encontró chxchx-tech. Configura CHXCHX_TECH_CLI o abre Studio desde el entorno del proyecto.")); });
        return;
    }
    m_promptFile = std::make_unique<QTemporaryFile>(QDir::temp().filePath(QStringLiteral("chxchx-assistant-XXXXXX.txt")));
    if (!m_promptFile->open() || m_promptFile->write(prompt.toUtf8()) < 0 || !m_promptFile->flush()) {
        return fail(QStringLiteral("No pude preparar la consulta para el asistente."));
    }
    m_output.clear();
    m_cancelled = false;
    m_timedOut = false;
    // `--cancel-on-stdin` keeps stdin open: writing "cancel" stops the agent cleanly, and closing it would too.
    QStringList arguments = {QStringLiteral("agent"), QStringLiteral("chat"), agentId, QStringLiteral("--path"), m_projectPath,
                             QStringLiteral("--lean"), QStringLiteral("--prompt-file"), m_promptFile->fileName(),
                             QStringLiteral("--cancel-on-stdin")};
    if (!model.isEmpty()) arguments << QStringLiteral("--model") << model;
    if (!effort.isEmpty()) arguments << QStringLiteral("--effort") << effort;
    m_process.start(m_cli, arguments);
    m_timeout.start(TimeoutMs);
}

void GitAssistant::cancel()
{
    if (!isBusy()) return;
    m_cancelled = true;
    m_process.write("cancel\n");
    QTimer::singleShot(KillAfterMs, &m_process, [this] { if (isBusy()) m_process.kill(); });
}

void GitAssistant::finish(int exitCode)
{
    m_output += m_process.readAllStandardOutput();
    GitAssistantResult result = parseOutput(m_output, exitCode, QString::fromUtf8(m_process.readAllStandardError()));
    result.cancelled = m_cancelled && !m_timedOut;
    if (m_timedOut) result.error = QStringLiteral("El asistente tardó demasiado y se detuvo.");
    m_promptFile.reset();
    m_timeout.stop();
    emit finished(result);
}

GitAssistantResult GitAssistant::parseOutput(const QByteArray &standardOutput, int exitCode, const QString &standardError)
{
    GitAssistantResult result;
    QStringList messages;
    for (const QByteArray &line : standardOutput.split('\n')) {
        const QJsonObject event = QJsonDocument::fromJson(line.trimmed()).object();
        if (event.value(QStringLiteral("schema")).toString() != QLatin1String("chxchx.agent-event")) continue;
        const QString type = event.value(QStringLiteral("type")).toString();
        if (type == QLatin1String("message")) messages << event.value(QStringLiteral("text")).toString();
        else if (type == QLatin1String("error")) result.error = event.value(QStringLiteral("message")).toString();
    }
    result.text = messages.join(QStringLiteral("\n\n")).trimmed();
    if (result.error.isEmpty() && exitCode != 0 && result.text.isEmpty()) {
        const QString detail = standardError.trimmed();
        result.error = detail.isEmpty() ? QStringLiteral("El asistente terminó con código %1.").arg(exitCode) : detail;
    }
    return result;
}
