#include "main_window.hpp"
#include "appearance_dialog.hpp"
#include "editor_theme.hpp"
#include "studio_icons.hpp"
#include "studio_theme.hpp"

#include <ScintillaEditBase.h>
#include <Scintilla.h>

#include <QAction>
#include <QActionGroup>
#include <QLabel>
#include <QMenu>
#include <QStatusBar>
#include <QTabWidget>
#include <QToolButton>

void MainWindow::buildAppearance()
{
    m_viewMenu->addSeparator();
    auto *appearance = m_viewMenu->addAction(studioIcon(QStringLiteral("studio-palette"), QStyle::SP_DesktopIcon),
        QStringLiteral("Apariencia…"), this, &MainWindow::showAppearanceDialog);
    appearance->setShortcut(QKeySequence(QStringLiteral("Ctrl+K, Ctrl+T")));
    m_themeMenu = m_viewMenu->addMenu(studioIcon(QStringLiteral("studio-palette"), QStyle::SP_DesktopIcon),
        QStringLiteral("Tema"));
    auto *group = new QActionGroup(this);
    for (const StudioTheme &theme : studioThemes()) {
        QAction *action = m_themeMenu->addAction(theme.name);
        action->setCheckable(true);
        action->setChecked(theme.id == ThemeManager::current().id);
        action->setData(theme.id);
        group->addAction(action);
        connect(action, &QAction::triggered, this, [id = theme.id] { ThemeManager::instance()->setTheme(id); });
    }

    m_statusPosition = new QLabel(this);
    m_statusLanguage = new QLabel(this);
    m_statusTheme = new QToolButton(this);
    m_statusTheme->setIcon(studioIcon(QStringLiteral("studio-palette"), QStyle::SP_DesktopIcon));
    m_statusTheme->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_statusTheme->setCursor(Qt::PointingHandCursor);
    m_statusTheme->setToolTip(QStringLiteral("Cambiar tema (Ctrl+K, Ctrl+T)"));
    connect(m_statusTheme, &QToolButton::clicked, this, &MainWindow::showAppearanceDialog);
    statusBar()->addPermanentWidget(m_statusPosition);
    statusBar()->addPermanentWidget(m_statusLanguage);
    statusBar()->addPermanentWidget(m_statusTheme);
    for (QTabWidget *tabs : editorTabGroups())
        connect(tabs, &QTabWidget::currentChanged, this, &MainWindow::updateEditorStatus);

    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, [this](const StudioTheme &theme) {
        applyThemeToEditors();
        for (QAction *action : m_themeMenu->actions()) action->setChecked(action->data().toString() == theme.id);
        updateEditorStatus();
    });
    updateEditorStatus();
}

void MainWindow::updateEditorStatus()
{
    if (m_statusTheme == nullptr) return;
    m_statusTheme->setText(ThemeManager::current().name);
    ScintillaEditBase *editor = currentEditor();
    m_statusPosition->setVisible(editor != nullptr);
    m_statusLanguage->setVisible(editor != nullptr);
    if (editor == nullptr) return;
    const auto position = editor->send(SCI_GETCURRENTPOS);
    const auto line = editor->send(SCI_LINEFROMPOSITION, static_cast<Scintilla::uptr_t>(position)) + 1;
    const auto column = editor->send(SCI_GETCOLUMN, static_cast<Scintilla::uptr_t>(position)) + 1;
    m_statusPosition->setText(QStringLiteral("Ln %1, Col %2").arg(line).arg(column));
    m_statusLanguage->setText(QStringLiteral("%1 · UTF-8").arg(languageNameForPath(currentFilePath())));
}

void MainWindow::applyThemeToEditors()
{
    for (QTabWidget *tabs : editorTabGroups()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index)))
                applyEditorTheme(editor, editor->property("filePath").toString(), ThemeManager::current());
        }
    }
}

void MainWindow::showAppearanceDialog()
{
    AppearanceDialog dialog(this);
    dialog.exec();
}
