#include "../src/integrations/workspace_terminal_widget.hpp"

#include <QApplication>
#include <QEventLoop>
#include <QLabel>
#include <QTimer>

#include <cstdio>

namespace {

bool terminalCommandFinishesSuccessfully()
{
    WorkspaceTerminalWidget terminal(QCoreApplication::applicationDirPath(), nullptr,
        {QCoreApplication::applicationFilePath(), QStringLiteral("--terminal-child")},
        QStringLiteral("Attach integrado de prueba"));
    terminal.show();

    QEventLoop loop;
    QTimer poll;
    poll.setInterval(25);
    QObject::connect(&poll, &QTimer::timeout, &loop, [&] {
        const auto labels = terminal.findChildren<QLabel *>();
        for (const auto *label : labels) {
            if (!label->text().contains(QStringLiteral("código 0"))) continue;
            loop.quit();
            return;
        }
    });
    QTimer::singleShot(5000, &loop, &QEventLoop::quit);
    poll.start();
    loop.exec();
    poll.stop();
    for (const auto *label : terminal.findChildren<QLabel *>())
        if (label->text().contains(QStringLiteral("código 0"))) return true;
    return false;
}

} // namespace

int main(int argc, char *argv[])
{
    if (argc > 1 && QString::fromLocal8Bit(argv[1]) == QStringLiteral("--terminal-child")) {
        std::fputs("ATTACH_SMOKE_OK\r\n", stdout);
        std::fflush(stdout);
        return 0;
    }

    QApplication app(argc, argv);
    return terminalCommandFinishesSuccessfully() ? 0 : 1;
}
