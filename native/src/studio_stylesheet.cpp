#include "studio_theme.hpp"

// Global widget stylesheet. Colours are @tokens@ expanded per theme, and
// @img:name@ points at themed PNGs rendered by the icon painter.
QString studioStyleSheet(const StudioTheme &theme)
{
    return ThemeManager::expand(QStringLiteral(R"(
        QMainWindow, QWidget { background-color: @background@; color: @text@; }
        QMainWindow::separator { background: @border@; width: 1px; height: 1px; }
        QToolTip { background-color: @surfaceRaised@; color: @text@; border: 1px solid @borderStrong@;
            border-radius: 6px; padding: 6px 9px; }

        QMenuBar { background-color: @surface@; border-bottom: 1px solid @border@; padding: 2px 6px; }
        QMenuBar::item { background: transparent; padding: 5px 10px; border-radius: 6px; color: @textMuted@; }
        QMenuBar::item:selected, QMenuBar::item:pressed { background: @surfaceHover@; color: @text@; }
        QMenu { background-color: @surfaceRaised@; border: 1px solid @borderStrong@; border-radius: 10px; padding: 6px; }
        QMenu::item { padding: 7px 30px 7px 12px; border-radius: 6px; color: @text@; background: transparent; }
        QMenu::item:selected { background-color: @accentSoft@; color: @selectedText@; }
        QMenu::item:disabled { color: @textFaint@; }
        QMenu::separator { height: 1px; background: @border@; margin: 5px 8px; }
        QMenu::icon { padding-left: 6px; }
        QMenu::indicator { width: 14px; height: 14px; left: 6px; }
        QMenu::indicator:checked { image: url(@img:check@); }

        QToolBar { background-color: @surface@; border: 0; border-bottom: 1px solid @border@;
            padding: 5px 10px; spacing: 2px; }
        QToolBar::separator { width: 1px; background: @border@; margin: 7px 8px; }
        QToolBar QToolButton { background: transparent; border: 1px solid transparent; border-radius: 8px;
            padding: 6px; margin: 0 1px; }
        QToolBar QToolButton:hover { background-color: @surfaceHover@; }
        QToolBar QToolButton:pressed { background-color: @accentSoft@; }
        QToolBar QToolButton:checked { background-color: @accentSoft@; border-color: @border@; }

        QPushButton { background-color: @surfaceRaised@; color: @text@; border: 1px solid @border@;
            border-radius: 8px; padding: 7px 14px; }
        QPushButton:hover { background-color: @surfaceHover@; border-color: @borderStrong@; }
        QPushButton:pressed { background-color: @accentSoft@; }
        QPushButton:default, QPushButton[primary="true"] { background-color: @accentStrong@; color: @onAccent@;
            border-color: @accentStrong@; font-weight: 600; }
        QPushButton:default:hover, QPushButton[primary="true"]:hover { background-color: @accent@; }
        QPushButton:disabled { color: @textFaint@; background-color: @surface@; border-color: @border@; }
        QToolButton { background: transparent; color: @text@; border: 0; border-radius: 6px; padding: 4px; }
        QToolButton:hover { background-color: @surfaceHover@; }
        QToolButton::menu-indicator { image: none; width: 0; }

        QLineEdit, QComboBox, QPlainTextEdit, QTextEdit, QSpinBox, QTextBrowser {
            background-color: @surface@; color: @text@; border: 1px solid @border@; border-radius: 8px;
            padding: 6px 9px; selection-background-color: @selection@; selection-color: @text@; }
        QLineEdit:hover, QComboBox:hover { border-color: @borderStrong@; }
        QLineEdit:focus, QComboBox:focus, QPlainTextEdit:focus, QTextEdit:focus { border-color: @accent@; }
        QComboBox { padding-right: 26px; }
        QComboBox::drop-down { border: 0; width: 24px; subcontrol-origin: padding; subcontrol-position: center right; }
        QComboBox::down-arrow { image: url(@img:chevron-down@); width: 12px; height: 12px; }
        QComboBox QAbstractItemView { background-color: @surfaceRaised@; color: @text@; border: 1px solid @borderStrong@;
            border-radius: 8px; padding: 4px; outline: 0; selection-background-color: @accentSoft@;
            selection-color: @selectedText@; }

        QListWidget, QTreeView, QListView, QTableView { background-color: transparent; border: 0; outline: 0;
            padding: 2px; }
        QListWidget::item, QListView::item { border-radius: 6px; padding: 7px 9px; margin: 1px 0; }
        QListWidget::item:hover, QListView::item:hover, QTreeView::item:hover { background-color: @surfaceHover@; }
        QListWidget::item:selected, QListView::item:selected, QTreeView::item:selected {
            background-color: @accentSoft@; color: @selectedText@; }
        QTreeView { show-decoration-selected: 1; }
        QTreeView::item { padding: 4px 2px; border-radius: 5px; }
        QTreeView::branch { background: transparent; }
        QTreeView::branch:has-children:closed { image: url(@img:chevron-right@); }
        QTreeView::branch:has-children:open { image: url(@img:chevron-down@); }
        QHeaderView::section { background-color: @surface@; color: @textMuted@; border: 0;
            border-bottom: 1px solid @border@; padding: 6px 8px; }

        QDockWidget { titlebar-close-icon: url(@img:close-small@); titlebar-normal-icon: url(@img:window-float@);
            color: @textMuted@; }
        QDockWidget::title { background-color: @surface@; color: @textMuted@; padding: 9px 12px;
            border-bottom: 1px solid @border@; font-weight: 600; }
        QDockWidget::close-button, QDockWidget::float-button { border: 0; border-radius: 5px; padding: 3px;
            background: transparent; icon-size: 14px; width: 18px; height: 18px; }
        QDockWidget::close-button:hover, QDockWidget::float-button:hover { background: @surfaceHover@; }
        QDockWidget > QWidget { background-color: @surface@; }

        QTabWidget::pane { border: 0; border-top: 1px solid @border@; background-color: @background@; top: -1px; }
        QTabBar { background-color: @surface@; qproperty-drawBase: 0; }
        QTabBar::tab { background: transparent; color: @textMuted@; min-width: 44px; max-width: 220px;
            padding: 8px 28px 8px 12px; margin: 0; border: 0; border-right: 1px solid @border@;
            border-bottom: 2px solid transparent; }
        QTabBar::tab:hover:!selected { background-color: @surfaceHover@; color: @text@; }
        QTabBar::tab:selected { background-color: @background@; color: @text@; border-bottom: 2px solid @accent@; }
        QTabBar::close-button { image: url(@img:close-small@); subcontrol-position: right; margin-right: 6px;
            width: 14px; height: 14px; border-radius: 4px; }
        QTabBar::close-button:hover { background-color: @surfaceHover@; }

        QStatusBar { background-color: @surface@; color: @textMuted@; border-top: 1px solid @border@; }
        QStatusBar::item { border: 0; }
        QStatusBar QLabel, QStatusBar QToolButton { color: @textMuted@; padding: 0 8px; background: transparent; }
        QStatusBar QToolButton:hover { color: @text@; background-color: @surfaceHover@; }

        QSplitter::handle { background-color: @border@; }
        QSplitter::handle:horizontal { width: 1px; }
        QSplitter::handle:vertical { height: 1px; }
        QScrollBar:vertical { background: transparent; width: 10px; margin: 2px; }
        QScrollBar:horizontal { background: transparent; height: 10px; margin: 2px; }
        QScrollBar::handle { background-color: @borderStrong@; border-radius: 4px; min-height: 28px; min-width: 28px; }
        QScrollBar::handle:hover { background-color: @textFaint@; }
        QScrollBar::add-line, QScrollBar::sub-line { width: 0; height: 0; border: 0; }
        QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }

        QGroupBox { background-color: @surface@; border: 1px solid @border@; border-radius: 12px;
            margin-top: 16px; padding: 14px 10px 10px 10px; font-weight: 600; }
        QGroupBox::title { subcontrol-origin: margin; left: 14px; padding: 0 6px; color: @textMuted@; }
        QCheckBox, QRadioButton { spacing: 8px; background: transparent; }
        QCheckBox::indicator { width: 16px; height: 16px; border: 1px solid @borderStrong@; border-radius: 4px;
            background: @surface@; }
        QCheckBox::indicator:checked { background: @accentStrong@; border-color: @accentStrong@;
            image: url(@img:check@); }
        QProgressBar { background-color: @surfaceHover@; border: 0; border-radius: 4px; height: 8px;
            text-align: center; color: transparent; }
        QProgressBar::chunk { background-color: @accent@; border-radius: 4px; }
        QDialog { background-color: @background@; }

        #pageIntro { color: @textMuted@; font-size: 14px; background: transparent; }
        #mutedText { color: @textMuted@; background: transparent; }
        #sectionTitle { color: @textMuted@; font-weight: 700; font-size: 12px; margin-top: 8px; background: transparent; }
        #dashboardSummary { background-color: @surface@; border: 1px solid @border@; border-left: 3px solid @accent@;
            border-radius: 10px; padding: 12px 14px; color: @text@; font-weight: 600; }
        #homeQuickActions QPushButton { text-align: left; padding: 9px 12px; }
        #areaActionBar { background-color: @surface@; border-bottom: 1px solid @border@; }
        #guideStepCard { background-color: @surface@; border: 1px solid @border@; border-radius: 12px; }
        #guideStepCard QLabel { background: transparent; }
        #guideNumber { color: @accent@; font-size: 18px; font-weight: 700; }
        #guideTitle { color: @text@; font-weight: 700; }
        #brandSubtitle { color: @accent@; letter-spacing: 3px; background: transparent; }
        #studioHeading { color: @accent@; font-weight: 800; letter-spacing: 2px; background: transparent; }
        #projectHeading { color: @textMuted@; padding-bottom: 4px; background: transparent; }
        #themeCard { background-color: @surface@; border: 1px solid @border@; border-radius: 12px; }
        #themeCard:hover { border-color: @borderStrong@; }
        #themeCard[selected="true"] { border: 2px solid @accent@; }
        #themeCard QLabel { background: transparent; }
        #themeName { color: @text@; font-weight: 700; font-size: 14px; }
        #themeDescription { color: @textMuted@; font-size: 12px; }
        #statusAccent { color: @accent@; font-weight: 600; }
    )"), theme);
}
