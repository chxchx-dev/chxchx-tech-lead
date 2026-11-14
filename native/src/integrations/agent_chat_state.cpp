#include "agent_chat_state.hpp"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>

namespace {

QStringList arguments(const QString &projectPath, const QString &agentId, const QString &flag)
{
    return {QStringLiteral("agent"), QStringLiteral("chat-state"), agentId, QStringLiteral("--path"), projectPath, flag};
}

} // namespace

void loadChatState(const QString &cliProgram, const QString &projectPath, const QString &agentId, QObject *context,
    const std::function<void(const ChatSavedState &)> &done)
{
    if (cliProgram.isEmpty()) return;
    auto *process = new QProcess(context);
    process->setWorkingDirectory(projectPath);
    QObject::connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), process,
        [process, done](int code, QProcess::ExitStatus) {
            const QByteArray output = process->readAllStandardOutput();
            process->deleteLater();
            if (code != 0) return;
            const QJsonObject state = QJsonDocument::fromJson(output.trimmed()).object();
            if (state.value(QStringLiteral("schema")).toString() != QLatin1String("chxchx.chat-state")) return;
            ChatSavedState saved;
            saved.sessionId = state.value(QStringLiteral("session_id")).toString();
            for (const auto &value : state.value(QStringLiteral("messages")).toArray()) {
                const QJsonObject message = value.toObject();
                const QString role = message.value(QStringLiteral("role")).toString();
                const QString text = message.value(QStringLiteral("text")).toString();
                if ((role == QLatin1String("user") || role == QLatin1String("assistant")) && !text.isEmpty()) {
                    saved.messages.append({role, text});
                }
            }
            if (!saved.messages.isEmpty()) done(saved);
        });
    QObject::connect(process, &QProcess::errorOccurred, process, [process](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) process->deleteLater(); // `finished` never comes
    });
    process->start(cliProgram, arguments(projectPath, agentId, QStringLiteral("--restore")));
}

void resetChatState(const QString &cliProgram, const QString &projectPath, const QString &agentId)
{
    if (cliProgram.isEmpty()) return;
    QProcess::startDetached(cliProgram, arguments(projectPath, agentId, QStringLiteral("--reset")), projectPath);
}
