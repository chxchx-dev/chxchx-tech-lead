#include "main_window.hpp"

#include <QApplication>
#include <QColor>
#include <QDir>
#include <QPalette>
#include <QStringList>

namespace {

void applyPalette(QApplication &application)
{
    QPalette palette = application.palette();
    palette.setColor(QPalette::Window, QColor(QStringLiteral("#282a36")));
    palette.setColor(QPalette::WindowText, QColor(QStringLiteral("#ececf1")));
    palette.setColor(QPalette::Base, QColor(QStringLiteral("#20222c")));
    palette.setColor(QPalette::AlternateBase, QColor(QStringLiteral("#2e303d")));
    palette.setColor(QPalette::Text, QColor(QStringLiteral("#ececf1")));
    palette.setColor(QPalette::Button, QColor(QStringLiteral("#303240")));
    palette.setColor(QPalette::ButtonText, QColor(QStringLiteral("#ececf1")));
    palette.setColor(QPalette::Highlight, QColor(QStringLiteral("#555970")));
    palette.setColor(QPalette::HighlightedText, QColor(QStringLiteral("#ffffff")));
    application.setPalette(palette);
}

} // namespace

int main(int argc, char *argv[])
{
    QApplication application(argc, argv);
    application.setApplicationName(QStringLiteral("ChxChx Studio"));
    application.setOrganizationName(QStringLiteral("ChxChx"));
    applyPalette(application);

    const QStringList arguments = application.arguments();
    const QString projectPath = arguments.size() > 1
        ? QDir(arguments.at(1)).absolutePath()
        : QDir::currentPath();

    MainWindow window(projectPath);
    window.show();
    return application.exec();
}
