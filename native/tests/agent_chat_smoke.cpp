#include "../src/integrations/agent_chat_model.hpp"
#include "../src/integrations/agent_chat_widget.hpp"

#include <QApplication>
#include <QElapsedTimer>
#include <QJsonDocument>
#include <QTemporaryDir>
#include <QThread>

#include <cstdio>
#include <iostream>
#include <string>

namespace {

QJsonObject event(QJsonObject fields)
{
    fields.insert(QStringLiteral("schema"), QStringLiteral("chxchx.agent-event"));
    fields.insert(QStringLiteral("schema_version"), 1);
    return fields;
}

void emitLine(const QJsonObject &fields)
{
    std::cout << QJsonDocument(event(fields)).toJson(QJsonDocument::Compact).toStdString() << std::endl;
}

// Stands in for `chxchx-tech agent chat` so the smoke test runs on every OS.
int fakeCli(const QStringList &arguments)
{
    const QString prompt = arguments.value(arguments.indexOf(QStringLiteral("--prompt")) + 1);
    const bool resumed = arguments.contains(QStringLiteral("--session"));
    emitLine({{QStringLiteral("type"), QStringLiteral("session")}, {QStringLiteral("session_id"), QStringLiteral("fake-1")}});
    if (prompt == QStringLiteral("dormir")) {
        std::string line;
        while (std::getline(std::cin, line) && line != "cancel") {}
        emitLine({{QStringLiteral("type"), QStringLiteral("done")}, {QStringLiteral("cancelled"), true}});
        return 0;
    }
    emitLine({{QStringLiteral("type"), QStringLiteral("message")}, {QStringLiteral("role"), QStringLiteral("assistant")},
        {QStringLiteral("text"), (resumed ? QStringLiteral("reanudado: ") : QStringLiteral("eco: ")) + prompt}});
    emitLine({{QStringLiteral("type"), QStringLiteral("done")}, {QStringLiteral("exit_code"), 0},
        {QStringLiteral("session_id"), QStringLiteral("fake-1")}});
    return 0;
}

bool waitIdle(const AgentChatWidget &chat, int timeoutMs = 15000)
{
    QElapsedTimer timer;
    timer.start();
    while (chat.isRunning() && timer.elapsed() < timeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
        QThread::msleep(10);
    }
    QCoreApplication::processEvents();
    return !chat.isRunning();
}

int modelChecks()
{
    AgentChatModel model;
    model.addUserMessage(QStringLiteral("Hola"));
    if (!model.apply(event({{QStringLiteral("type"), QStringLiteral("session")}, {QStringLiteral("session_id"), QStringLiteral("s1")}})))
        return 10;
    model.apply(event({{QStringLiteral("type"), QStringLiteral("message")}, {QStringLiteral("text"), QStringLiteral("**listo** <script>")}}));
    model.apply(event({{QStringLiteral("type"), QStringLiteral("tool")}, {QStringLiteral("id"), QStringLiteral("t1")},
        {QStringLiteral("kind"), QStringLiteral("command")}, {QStringLiteral("title"), QStringLiteral("ls")},
        {QStringLiteral("status"), QStringLiteral("running")}}));
    model.apply(event({{QStringLiteral("type"), QStringLiteral("tool")}, {QStringLiteral("id"), QStringLiteral("t1")},
        {QStringLiteral("status"), QStringLiteral("completed")}, {QStringLiteral("output"), QStringLiteral("app.py")}}));
    model.apply(event({{QStringLiteral("type"), QStringLiteral("usage")}, {QStringLiteral("input_tokens"), 10},
        {QStringLiteral("output_tokens"), 2}}));
    if (model.apply(QJsonObject{{QStringLiteral("schema"), QStringLiteral("otro")}, {QStringLiteral("type"), QStringLiteral("message")}}))
        return 11;
    if (model.entries().size() != 3 || model.sessionId() != QStringLiteral("s1")) return 12;
    const auto &tool = model.entries().at(2);
    if (tool.status != QStringLiteral("completed") || tool.output != QStringLiteral("app.py") || tool.text != QStringLiteral("ls"))
        return 13;
    const QString html = model.renderHtml();
    if (!html.contains(QStringLiteral("listo")) || html.contains(QStringLiteral("<script>")) || !html.contains(QStringLiteral("✓")))
        return 14;
    if (!model.usageSummary().contains(QStringLiteral("10"))) return 15;
    if (!AgentChatWidget::supportsProgram(QStringLiteral("C:\\npm\\claude.cmd"))
        || AgentChatWidget::supportsProgram(QStringLiteral("opencode"))) return 16;
    return 0;
}

} // namespace

int main(int argc, char **argv)
{
    // Read argv before QApplication: Qt consumes options such as --session.
    QStringList arguments;
    for (int index = 0; index < argc; ++index) arguments << QString::fromLocal8Bit(argv[index]);
    if (arguments.size() > 2 && arguments.at(1) == QStringLiteral("agent") && arguments.at(2) == QStringLiteral("chat"))
        return fakeCli(arguments);
    QApplication app(argc, argv);

    if (const int failure = modelChecks()) return failure;

    QTemporaryDir project;
    auto *chat = new AgentChatWidget(project.path(), QStringLiteral("codex"), QCoreApplication::applicationFilePath());
    chat->sendMessage(QStringLiteral("hola"));
    if (!waitIdle(*chat)) return 20;
    const auto &first = chat->model().entries();
    if (first.size() != 2 || first.at(1).text != QStringLiteral("eco: hola")) return 21;
    if (chat->model().sessionId() != QStringLiteral("fake-1")) return 22;

    chat->sendMessage(QStringLiteral("sigue"));
    if (!waitIdle(*chat)) return 23;
    if (chat->model().entries().constLast().text != QStringLiteral("reanudado: sigue")) return 24;

    chat->sendMessage(QStringLiteral("dormir"));
    QCoreApplication::processEvents();
    if (!chat->isRunning()) return 25;
    QElapsedTimer closing;
    closing.start();
    delete chat; // Closing the tab must stop the running turn promptly.
    if (closing.elapsed() > 6000) return 26;
    return 0;
}
