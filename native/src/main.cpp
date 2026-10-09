#include "main_window.hpp"

#include <QApplication>
#include <QColor>
#include <QDir>
#include <QIcon>
#include <QPalette>
#include <QStringList>

namespace {

void applyPalette(QApplication &application)
{
    QPalette palette = application.palette();
    palette.setColor(QPalette::Window, QColor(QStringLiteral("#0b1220")));
    palette.setColor(QPalette::WindowText, QColor(QStringLiteral("#e6edf7")));
    palette.setColor(QPalette::Base, QColor(QStringLiteral("#0e1728")));
    palette.setColor(QPalette::AlternateBase, QColor(QStringLiteral("#111d30")));
    palette.setColor(QPalette::Text, QColor(QStringLiteral("#e6edf7")));
    palette.setColor(QPalette::Button, QColor(QStringLiteral("#17243a")));
    palette.setColor(QPalette::ButtonText, QColor(QStringLiteral("#e6edf7")));
    palette.setColor(QPalette::Highlight, QColor(QStringLiteral("#087e8b")));
    palette.setColor(QPalette::HighlightedText, QColor(QStringLiteral("#ffffff")));
    application.setPalette(palette);
    application.setStyleSheet(QStringLiteral(R"(
        QMainWindow, QWidget { background-color: #0b1220; color: #e6edf7; }
        QMenuBar, QMenu, QToolBar { background-color: #0e1728; border: 0; }
        QMenuBar { border-bottom: 1px solid #1d2b40; }
        QToolBar { spacing: 6px; padding: 7px 10px; border-bottom: 1px solid #1d2b40; }
        QToolButton, QPushButton {
            background-color: #17243a; color: #dce8f7; border: 1px solid #263a55;
            border-radius: 7px; padding: 7px 11px;
        }
        QToolButton:hover, QPushButton:hover { background-color: #203552; border-color: #20c5d4; }
        QToolButton:checked { background-color: #123846; border-color: #20c5d4; }
        QPushButton:default { background-color: #087e8b; color: #ffffff; border-color: #21c6d5; }
        QPushButton:disabled { color: #64748b; background-color: #111a29; border-color: #1d2b40; }
        QLineEdit, QComboBox, QPlainTextEdit, QListWidget, QTreeView {
            background-color: #0e1728; color: #dce8f7; border: 1px solid #24364f;
            border-radius: 6px; padding: 6px; selection-background-color: #087e8b;
            selection-color: #ffffff;
        }
        QLineEdit:focus, QComboBox:focus { border: 1px solid #20c5d4; }
        QListWidget { padding: 5px; }
        QListWidget::item { border-radius: 5px; padding: 7px 8px; }
        QListWidget::item:hover { background-color: #15263c; }
        QListWidget::item:selected { background-color: #123846; color: #8cf2f4; border-left: 2px solid #20c5d4; }
        QTreeView { alternate-background-color: #101a2a; show-decoration-selected: 1; padding: 4px; }
        QTreeView::item { padding: 5px 4px; border-radius: 4px; }
        QTreeView::item:hover { background-color: #15263c; }
        QTreeView::item:selected { background-color: #123846; color: #8cf2f4; border-left: 2px solid #20c5d4; }
        QTreeView::branch { background-color: transparent; }
        QDockWidget::title { background-color: #101c2e; color: #9fb4ce; padding: 9px 11px; border-bottom: 1px solid #1d2b40; }
        QTabWidget::pane { border: 1px solid #1d2b40; background-color: #0b1220; }
        QTabBar { background-color: #0e1728; qproperty-drawBase: 0; }
        QTabBar::tab {
            background-color: #101a2a; color: #91a7c2; min-width: 44px; max-width: 180px;
            min-height: 27px; padding: 3px 21px 3px 7px; margin: 2px 1px 0 1px;
            text-align: left; font-size: 12px;
            border: 1px solid #1d2b40; border-top-left-radius: 6px; border-top-right-radius: 6px;
        }
        QTabBar::close-button {
            subcontrol-origin: padding; subcontrol-position: right center;
            right: 5px; width: 12px; height: 12px; margin: 0;
        }
        QTabBar::close-button:hover { background-color: #743846; border-radius: 7px; }
        QTabBar::tab:hover { background-color: #17283d; color: #dce8f7; }
        QTabBar::tab:selected { background-color: #14283c; color: #8cf2f4; border-color: #24566a; border-bottom: 2px solid #20c5d4; }
        QStatusBar { background-color: #0e1728; color: #91a7c2; border-top: 1px solid #1d2b40; }
        QSplitter::handle { background-color: #1d2b40; }
        QScrollBar:vertical { background: #0b1220; width: 10px; }
        QScrollBar::handle:vertical { background: #2b405b; border-radius: 4px; min-height: 24px; }
    )"));
}

} // namespace

int main(int argc, char *argv[])
{
    QApplication application(argc, argv);
    application.setApplicationName(QStringLiteral("ChxChx Studio"));
    application.setOrganizationName(QStringLiteral("ChxChx"));
    application.setWindowIcon(QIcon(QStringLiteral(":/brand/logo-min.png")));
    applyPalette(application);

    const QStringList arguments = application.arguments();
    const QString projectPath = arguments.size() > 1
        ? QDir(arguments.at(1)).absolutePath()
        : QDir::currentPath();

    MainWindow window(projectPath);
    window.show();
    return application.exec();
}
