#include "../src/integrations/workspace_terminal_widget.hpp"
#include "../src/integrations/vt_terminal_widget.hpp"

#include <QApplication>
#include <QEventLoop>
#include <QKeyEvent>
#include <QLabel>
#include <QTimer>

#include <cstdio>
#include <iostream>
#include <string>

namespace {

bool terminalCommandFinishesSuccessfully()
{
    WorkspaceTerminalWidget terminal(QCoreApplication::applicationDirPath(), nullptr,
        {QCoreApplication::applicationFilePath(), QStringLiteral("--terminal-child")},
        QStringLiteral("Attach integrado de prueba"));
    terminal.resize(720, 440);
    terminal.show();
    auto *emulator = terminal.findChild<VtTerminalWidget *>();
    if (emulator == nullptr) return false;

    int resizedColumns = 0;
    int resizedRows = 0;
    QObject::connect(emulator, &VtTerminalWidget::terminalResized, &terminal,
        [&resizedColumns, &resizedRows](int columns, int rows) {
            resizedColumns = columns;
            resizedRows = rows;
        });

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
    QTimer::singleShot(400, &loop, [&] {
        terminal.resize(500, 340);
        QCoreApplication::processEvents();
        QKeyEvent keyG(QEvent::KeyPress, Qt::Key_G, Qt::NoModifier, QStringLiteral("g"));
        QKeyEvent keyO(QEvent::KeyPress, Qt::Key_O, Qt::NoModifier, QStringLiteral("o"));
        QKeyEvent enter(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
        QApplication::sendEvent(emulator, &keyG);
        QApplication::sendEvent(emulator, &keyO);
        QApplication::sendEvent(emulator, &enter);
    });
    QTimer::singleShot(5000, &loop, &QEventLoop::quit);
    poll.start();
    loop.exec();
    poll.stop();
    if (resizedColumns <= 0 || resizedRows <= 0) return false;
    for (const auto *label : terminal.findChildren<QLabel *>())
        if (label->text().contains(QStringLiteral("código 0"))) return true;
    return false;
}

} // namespace

int main(int argc, char *argv[])
{
    if (argc > 1 && QString::fromLocal8Bit(argv[1]) == QStringLiteral("--terminal-child")) {
        std::fputs("READY\r\n", stdout);
        for (int line = 0; line < 80; ++line)
            std::fprintf(stdout, "long-interactive-output-%03d keeps the PTY active\r\n", line);
        std::fflush(stdout);
        std::string input;
        std::getline(std::cin, input);
        std::fprintf(stdout, "INPUT_OK=%s\r\n", input.c_str());
        return input == "go" ? 0 : 2;
    }

    QApplication app(argc, argv);
    return terminalCommandFinishesSuccessfully() ? 0 : 1;
}
