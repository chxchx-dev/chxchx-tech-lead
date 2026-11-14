#include "main_window.hpp"
#include "studio_theme.hpp"

#include <QApplication>
#include <QDir>
#include <QIcon>
#include <QStringList>
#include <QTimer>

#include <cstdio>
#include <cstring>

#ifdef _WIN32
#include <windows.h>
#endif

#ifndef CHXCHX_STUDIO_VERSION
#define CHXCHX_STUDIO_VERSION "desconocida"
#endif

int main(int argc, char *argv[])
{
    // Answered before Qt starts so installers and scripts can check the binary without a display.
    const bool wantsVersion = argc > 1 && std::strcmp(argv[1], "--version") == 0;
    const bool wantsHelp = argc > 1 && (std::strcmp(argv[1], "--help") == 0 || std::strcmp(argv[1], "-h") == 0);
#ifdef _WIN32
    // A GUI-subsystem program has no console of its own: borrow the parent's unless output is redirected.
    if ((wantsVersion || wantsHelp) && GetStdHandle(STD_OUTPUT_HANDLE) == nullptr && AttachConsole(ATTACH_PARENT_PROCESS)) {
        std::freopen("CONOUT$", "w", stdout);
        std::freopen("CONOUT$", "w", stderr);
    }
#endif
    if (wantsVersion) {
        std::printf("ChxChx Studio %s\n", CHXCHX_STUDIO_VERSION);
        return 0;
    }
    if (wantsHelp) {
        std::printf("Uso: chxchx-studio [carpeta-del-proyecto [archivos...]]\n       chxchx-studio --version\n");
        return 0;
    }
    QApplication application(argc, argv);
    application.setApplicationName(QStringLiteral("ChxChx Studio"));
    application.setOrganizationName(QStringLiteral("ChxChx"));
    application.setWindowIcon(QIcon(QStringLiteral(":/brand/logo-min.png")));
    applyStudioTheme(application);

    const QStringList arguments = application.arguments();
    const QString projectPath = arguments.size() > 1
        ? QDir(arguments.at(1)).absolutePath()
        : QDir::currentPath();

    MainWindow window(projectPath);
    window.openFiles(arguments.mid(2));
    window.show();
    // Developer aid for docs and visual QA: save a capture of the window and quit.
    const QString screenshot = qEnvironmentVariable("CHXCHX_STUDIO_SCREENSHOT");
    if (!screenshot.isEmpty()) {
        // Optional WIDTHxHEIGHT to check layouts at other window sizes.
        const QStringList size = qEnvironmentVariable("CHXCHX_STUDIO_SCREENSHOT_SIZE").split(QLatin1Char('x'));
        if (size.size() == 2) window.resize(size.at(0).toInt(), size.at(1).toInt());
        const QString area = qEnvironmentVariable("CHXCHX_STUDIO_SCREENSHOT_AREA");
        if (!area.isEmpty()) window.showArea(area);
        const QString tool = qEnvironmentVariable("CHXCHX_STUDIO_SCREENSHOT_TOOL");
        if (!tool.isEmpty()) window.showTool(tool.section(QLatin1Char(':'), 0, 0), tool.section(QLatin1Char(':'), 1));
        const int delay = qEnvironmentVariableIntValue("CHXCHX_STUDIO_SCREENSHOT_DELAY_MS");
        QTimer::singleShot(delay > 0 ? delay : 2500, &window, [&window, &application, screenshot] {
            window.grab().save(screenshot);
            // A dialog on screen (the Git assistant's advice) gets its own capture next to the window's.
            for (QWidget *widget : QApplication::topLevelWidgets()) {
                if (widget->objectName() == QStringLiteral("gitAdviceDialog") && widget->isVisible())
                    widget->grab().save(screenshot + QStringLiteral(".dialog.png"));
            }
            application.quit();
        });
    }
    return application.exec();
}
