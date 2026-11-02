#include "main_window.hpp"
#include "studio_theme.hpp"

#include <QApplication>
#include <QDir>
#include <QIcon>
#include <QStringList>

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
    window.show();
    return application.exec();
}
