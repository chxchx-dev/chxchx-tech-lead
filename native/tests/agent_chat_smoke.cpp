#include "../src/integrations/agent_chat_markdown.hpp"
#include "../src/integrations/agent_chat_model.hpp"
#include "../src/integrations/agent_chat_usage.hpp"
#include "../src/integrations/agent_chat_view.hpp"
#include "../src/integrations/agent_chat_widget.hpp"

#include <QApplication>
#include <QElapsedTimer>
#include <QImage>
#include <QJsonArray>
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

// Stands in for `chxchx-tech bridge agent-options`: "claude" is not installed.
int fakeOptions(const QStringList &arguments)
{
    const bool claude = arguments.value(arguments.indexOf(QStringLiteral("--agent")) + 1) == QStringLiteral("claude");
    QJsonObject payload{{QStringLiteral("schema"), QStringLiteral("chxchx.agent-options")},
        {QStringLiteral("schema_version"), 1}, {QStringLiteral("available"), !claude},
        {QStringLiteral("install_hint"), QStringLiteral("curl -fsSL https://claude.ai/install.sh | bash")},
        {QStringLiteral("default_model"), QStringLiteral("gpt-b")}, {QStringLiteral("default_effort"), QString()},
        {QStringLiteral("models"), QJsonArray{QJsonObject{{QStringLiteral("id"), QStringLiteral("gpt-a")},
            {QStringLiteral("label"), QStringLiteral("GPT A")}, {QStringLiteral("default_effort"), QStringLiteral("low")},
            {QStringLiteral("efforts"), QJsonArray{QStringLiteral("low"), QStringLiteral("ultra")}}}}}};
    std::cout << QJsonDocument(payload).toJson(QJsonDocument::Compact).toStdString() << std::endl;
    return 0;
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
    const QString extras = QStringLiteral(" [%1 img, modelo=%2]").arg(arguments.count(QStringLiteral("--image")))
        .arg(arguments.contains(QStringLiteral("--model"))
            ? arguments.value(arguments.indexOf(QStringLiteral("--model")) + 1) : QStringLiteral("-"));
    emitLine({{QStringLiteral("type"), QStringLiteral("message")}, {QStringLiteral("role"), QStringLiteral("assistant")},
        {QStringLiteral("text"), (resumed ? QStringLiteral("reanudado: ") : QStringLiteral("eco: ")) + prompt
            + (arguments.contains(QStringLiteral("--image")) ? extras : QString())}});
    emitLine({{QStringLiteral("type"), QStringLiteral("usage")}, {QStringLiteral("input_tokens"), 1000},
        {QStringLiteral("output_tokens"), 10}});
    emitLine({{QStringLiteral("type"), QStringLiteral("limits")}, {QStringLiteral("context_tokens"), 50000},
        {QStringLiteral("context_window"), 200000}, {QStringLiteral("plan"), QStringLiteral("plus")},
        {QStringLiteral("primary"), QJsonObject{{QStringLiteral("used_percent"), 12.0},
            {QStringLiteral("window_minutes"), 300}, {QStringLiteral("resets_at"), 1793633900}}}});
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
    if (model.entries().at(2).revision != 1) return 14;
    const QString html = chatMarkdownToHtml(QStringLiteral("**listo** <script>alert(1)</script>"));
    if (!html.contains(QStringLiteral("listo")) || html.contains(QStringLiteral("<script>"))) return 19;
    const QList<ChatSegment> segments = splitFencedCode(
        QStringLiteral("Mira:\n```python\nprint(1)\n```\nListo."));
    if (segments.size() != 3 || !segments.at(1).code || segments.at(1).language != QStringLiteral("python")
        || segments.at(1).text != QStringLiteral("print(1)") || segments.at(2).text != QStringLiteral("Listo.")) return 34;
    if (!model.usageSummary().contains(QStringLiteral("10"))) return 15;
    model.apply(event({{QStringLiteral("type"), QStringLiteral("usage")}, {QStringLiteral("input_tokens"), 5},
        {QStringLiteral("output_tokens"), 1}, {QStringLiteral("cost_usd"), 0.25}}));
    model.apply(event({{QStringLiteral("type"), QStringLiteral("limits")}, {QStringLiteral("context_tokens"), 100000},
        {QStringLiteral("context_window"), 200000}, {QStringLiteral("secondary"), QJsonObject{
            {QStringLiteral("used_percent"), 90.0}, {QStringLiteral("window_minutes"), 10080}}}}));
    if (model.usage().inputTokens != 15 || !model.usage().hasCost) return 17;
    ChatUsageBar usageBar;
    usageBar.setUsage(model.usage(), model.usageSummary());
    const QString usageText = usageBar.summary();
    if (!usageText.contains(QStringLiteral("Contexto 50%")) || !usageText.contains(QStringLiteral("Semana 90%"))
        || !usageText.contains(QStringLiteral("0.2500")) || usageText.contains(QStringLiteral("5 h"))) return 18;

    AgentChatView view;
    view.sync(model);
    if (view.cardCount() != 3) return 35;
    model.apply(event({{QStringLiteral("type"), QStringLiteral("tool")}, {QStringLiteral("id"), QStringLiteral("t1")},
        {QStringLiteral("status"), QStringLiteral("failed")}}));
    view.sync(model);
    if (view.cardCount() != 3) return 36;
    model.clear();
    view.sync(model);
    if (view.cardCount() != 0) return 37;
    if (!AgentChatWidget::supportsProgram(QStringLiteral("C:\\npm\\claude.cmd"))
        || !AgentChatWidget::supportsProgram(QStringLiteral("opencode"))
        || AgentChatWidget::supportsProgram(QStringLiteral("aider"))) return 16;
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
    if (arguments.size() > 2 && arguments.at(1) == QStringLiteral("bridge"))
        return fakeOptions(arguments);
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

    QTemporaryDir images;
    const QString image = images.filePath(QStringLiteral("captura.png"));
    QImage pixel(4, 4, QImage::Format_RGB32);
    pixel.fill(Qt::red);
    pixel.save(image);
    chat->sendMessage(QStringLiteral("mira"), {image});
    if (!waitIdle(*chat)) return 27;
    const auto &withImage = chat->model().entries();
    if (withImage.at(withImage.size() - 2).images != QStringList{image}) return 28;
    if (!withImage.constLast().text.contains(QStringLiteral("[1 img, modelo=-]"))) return 29;
    if (chat->model().usage().contextWindow != 200000 || chat->model().usage().primary.usedPercent != 12.0) return 30;

    auto *missing = new AgentChatWidget(project.path(), QStringLiteral("claude"), QCoreApplication::applicationFilePath());
    QElapsedTimer loading;
    loading.start();
    while (missing->isAvailable() && loading.elapsed() < 10000) QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
    if (missing->isAvailable()) return 31;
    if (!missing->model().entries().constLast().text.contains(QStringLiteral("install.sh"))) return 32;
    missing->sendMessage(QStringLiteral("hola"));
    if (missing->isRunning() || missing->model().entries().size() != 1) return 33;
    delete missing;

    chat->sendMessage(QStringLiteral("dormir"));
    QCoreApplication::processEvents();
    if (!chat->isRunning()) return 25;
    QElapsedTimer closing;
    closing.start();
    delete chat; // Closing the tab must stop the running turn promptly.
    if (closing.elapsed() > 6000) return 26;
    return 0;
}
