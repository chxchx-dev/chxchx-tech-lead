// Git in Studio: the sidebar tab, diff tabs and the file-diff shortcut. All git work is in src/git/.
#include "main_window.hpp"

#include "git/git_diff_view.hpp"
#include "studio_icons.hpp"
#include <QStyle>
#include "integrations/agent_chat_widget.hpp"
#include "integrations/bridge_client.hpp"
#include "main_window_areas.hpp"
#include <QComboBox>
#include "git/git_panel.hpp"
#include "git/git_repository.hpp"
#include "git/git_tree_delegate.hpp"

#include <QApplication>
#include <QDir>
#include <QDockWidget>
#include <QGuiApplication>
#include <QStatusBar>
#include <QTabWidget>
#include <QTreeView>

namespace {

constexpr int FilesTab = 0;
constexpr int GitTab = 1;
const char *DiffKeyProperty = "gitDiffKey";

} // namespace

void MainWindow::buildGitPanel(QWidget *explorer)
{
    m_sidebarTabs = new QTabWidget(this);
    m_sidebarTabs->setDocumentMode(true);
    m_sidebarTabs->setObjectName(QStringLiteral("sidebarTabs"));
    m_sidebarTabs->setIconSize(QSize(16, 16));
    m_sidebarTabs->addTab(explorer, studioIcon(QStringLiteral("folders"), QStyle::SP_DirIcon), QStringLiteral("Archivos"));
    m_gitPanel = new GitPanel(m_projectPath, m_sidebarTabs);
    m_sidebarTabs->addTab(m_gitPanel, studioIcon(QStringLiteral("studio-git"), QStyle::SP_FileIcon), QStringLiteral("Git"));
    m_projectDock->setWidget(m_sidebarTabs);

    // The file explorer shows Git state: coloured names, a letter per file and a dot on folders.
    auto *treeDelegate = new GitTreeDelegate(m_projectTree);
    m_projectTree->setItemDelegate(treeDelegate);
    connect(m_gitPanel, &GitPanel::explorerStatusChanged, this, [this, treeDelegate](const GitExplorerStatus &status) {
        treeDelegate->setStatus(status);
        m_projectTree->viewport()->update();
    });
    connect(m_gitPanel, &GitPanel::openDiffRequested, this, [this](const GitDiffRequest &request) { openGitDiff(request); });
    connect(m_gitPanel, &GitPanel::openFileRequested, this, [this](const QString &relativePath) {
        openPath(QDir(m_gitPanel->repository()->root()).filePath(relativePath));
    });
    connect(m_gitPanel, &GitPanel::changeCountChanged, this, [this](int count) {
        m_sidebarTabs->setTabText(GitTab, count > 0 ? QStringLiteral("Git (%1)").arg(count) : QStringLiteral("Git"));
    });
    connect(m_sidebarTabs, &QTabWidget::currentChanged, this, [this](int index) {
        if (index == GitTab) m_gitPanel->refresh();
    });
    connect(qApp, &QGuiApplication::applicationStateChanged, this, [this](Qt::ApplicationState state) {
        if (state == Qt::ApplicationActive) m_gitPanel->scheduleRefresh();
    });
}

void MainWindow::configureGitAssistant()
{
    // Called once the bridge client exists (the panel is built earlier, while the layout is still being created).
    // The assistant answers through the project's own agents: the ones the Home selector lists that Studio can drive.
    m_gitPanel->configureAssistant(m_bridgeClient->program(), [this] {
        QList<GitAssistantAgent> agents;
        if (m_homeAgentSelector == nullptr) return agents;
        for (int index = 0; index < m_homeAgentSelector->count(); ++index) {
            const QString program = m_homeAgentSelector->itemData(index, Qt::UserRole + 1).toString();
            if (!m_homeAgentSelector->itemData(index, AddAgentRole).toString().isEmpty() || program.isEmpty()
                || m_homeAgentSelector->itemData(index, Qt::UserRole + 4).toBool() || !AgentChatWidget::supportsProgram(program)) continue;
            agents.append({m_homeAgentSelector->itemText(index), m_homeAgentSelector->itemText(index)});
        }
        return agents;
    }, [this] {
        // The agent of the chat in front of the person, if there is one.
        const QTabWidget *tabs = activeEditorTabs();
        const auto *chat = tabs != nullptr ? qobject_cast<AgentChatWidget *>(tabs->currentWidget()) : nullptr;
        return chat != nullptr ? chat->agentId() : QString();
    });
}

void MainWindow::showGitPanel()
{
    m_projectDock->show();
    m_sidebarTabs->setCurrentIndex(GitTab);
    m_gitPanel->refresh();
}

void MainWindow::openGitDiff(const GitDiffRequest &request)
{
    QTabWidget *tabs = activeEditorTabs();
    const QString key = request.key();
    for (int index = 0; index < tabs->count(); ++index) {
        if (auto *existing = qobject_cast<GitDiffView *>(tabs->widget(index)); existing != nullptr
            && existing->property(DiffKeyProperty).toString() == key) {
            tabs->setCurrentIndex(index);
            existing->reload();
            return;
        }
    }
    auto *view = new GitDiffView(m_gitPanel->repository(), request, tabs);
    view->setProperty(DiffKeyProperty, key);
    connect(view, &GitDiffView::openFileRequested, this, [this](const QString &relativePath) {
        openPath(QDir(m_gitPanel->repository()->root()).filePath(relativePath));
    });
    const int tab = tabs->addTab(view, request.tabTitle());
    tabs->setTabToolTip(tab, QStringLiteral("%1\n%2").arg(request.path, request.badge()));
    tabs->setCurrentWidget(view);
}

void MainWindow::showCurrentFileDiff()
{
    const QString path = currentFilePath();
    if (path.isEmpty()) {
        statusBar()->showMessage(QStringLiteral("Abre un archivo para consultar su diff Git."), 4000);
        return;
    }
    if (!isProjectFilePathSafe(path)) {
        statusBar()->showMessage(QStringLiteral("El archivo está fuera del proyecto actual."), 4000);
        return;
    }
    if (!m_gitPanel->isRepository()) {
        statusBar()->showMessage(QStringLiteral("Este proyecto no es un repositorio Git."), 4000);
        return;
    }
    GitDiffRequest request;
    request.source = GitDiffRequest::Source::Head;
    request.path = QDir(m_gitPanel->repository()->root()).relativeFilePath(path);
    openGitDiff(request);
}
