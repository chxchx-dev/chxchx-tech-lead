#include "main_window.hpp"
#include "studio_theme.hpp"

#include <QApplication>
#include <QDir>
#include <QIcon>
#include <QStringList>
#include <QTimer>

int main(int argc, char *argv[])
{
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
        const int delay = qEnvironmentVariableIntValue("CHXCHX_STUDIO_SCREENSHOT_DELAY_MS");
        QTimer::singleShot(delay > 0 ? delay : 2500, &window, [&window, &application, screenshot] {
            window.grab().save(screenshot);
            application.quit();
        });
    }
    return application.exec();
}
