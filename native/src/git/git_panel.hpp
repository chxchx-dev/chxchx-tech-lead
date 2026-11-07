#pragma once

#include "git_diff_view.hpp"
#include "git_assistant_controller.hpp"
#include "git_explorer.hpp"
#include "git_repository.hpp"
#include "git_types.hpp"

#include <QTimer>
#include <QWidget>

class GitBranchesView;
class GitChangesView;
class GitCommitPlanView;
class GitHistoryView;
class QLabel;
class QPushButton;
class QStackedWidget;
class QTabWidget;
class QToolButton;

// The Git tab next to the file explorer: changes, history and branches for the project's repository.
class GitPanel final : public QWidget {
    Q_OBJECT
public:
    explicit GitPanel(QString projectPath, QWidget *parent = nullptr);
    ~GitPanel() override;

    GitRepository *repository() const { return m_repository; }
    void refresh();
    void scheduleRefresh();
    int changeCount() const { return m_changeCount; }
    bool isRepository() const { return m_isRepository; }
    // For tests.
    GitChangesView *changesView() const { return m_changes; }
    GitHistoryView *historyView() const { return m_history; }
    GitBranchesView *branchesView() const { return m_branches; }
    QString toastText() const;
    QString branchText() const;
    QString syncText() const;
    QString stateText() const;
    bool refreshing() const { return m_refreshing; }
    void showTab(int index);
    // The assistant: `agents` lists the agents that can answer, `preferred` the one in use (may be empty).
    void configureAssistant(const QString &cliProgram, GitAssistantController::AgentProvider agents, GitAssistantController::PreferredProvider preferred);
    GitAssistantController *assistant() const { return m_assistant; }
    void adviseNow(GitAdviceKind kind, const QString &lastError = QString());
    GitCommitPlanView *commitPlan() const { return m_plan; }
    // Tests: show no dialogs; the advice is kept in `lastAdvice()` instead.
    void setAdviceDialogsEnabled(bool enabled) { m_adviceDialogs = enabled; }
    QString lastAdvice() const { return m_lastAdvice; }
    bool toastOffersAdvice() const;
    // Sync with the remote. Without arguments `syncNow` does whatever the button says.
    void syncNow();
    void pullNow(GitPullMode mode = GitPullMode::FastForwardOnly);
    void pushNow();
    void fetchNow();
    void cancelSync();
    bool syncing() const { return m_syncing; }
    // Tests: skip every dialog (no confirmation before sending, no choice when branches diverge).
    void setConfirmations(bool enabled) { m_confirm = enabled; }

signals:
    void openDiffRequested(const GitDiffRequest &request);
    void openFileRequested(const QString &relativePath);
    void changeCountChanged(int count);
    void explorerStatusChanged(const GitExplorerStatus &status);

private:
    void preparePlanGroup(int index);
    void applyStatus(const GitStatus &status);
    void showNotRepository(bool gitMissing);
    void loadBranches();
    void showToast(const QString &text, bool error, GitProblem problem = GitProblem::Other, bool offerAdvice = true);
    void updateGuidance();
    void rebuildBranchMenu();
    void tabChanged(int index);
    void updateSyncButton();
    void beginSync(const QString &text);
    void endSync();
    QString remoteName() const;
    bool confirmPush(const QString &text);

    QString m_projectPath;
    GitRepository *m_repository = nullptr;
    bool m_isRepository = false;
    bool m_detected = false;
    bool m_refreshing = false;
    bool m_dirty = false;
    bool m_historyStale = true;
    bool m_branchesStale = true;
    int m_changeCount = 0;
    QString m_lastHead;
    QList<GitBranch> m_branchList;
    GitStatus m_status;
    QTimer m_debounce;
    QTimer m_poll;
    QTimer m_toastTimer;
    QToolButton *m_branchButton = nullptr;
    QToolButton *m_syncButton = nullptr;
    QAction *m_pullAction = nullptr;
    QAction *m_pushAction = nullptr;
    QAction *m_fetchAction = nullptr;
    QStringList m_remotes;
    bool m_syncing = false;
    bool m_confirm = true;
    QStackedWidget *m_stack = nullptr;
    QLabel *m_notRepository = nullptr;
    QPushButton *m_init = nullptr;
    QTabWidget *m_tabs = nullptr;
    QLabel *m_toast = nullptr;
    QWidget *m_toastBox = nullptr;
    QLabel *m_toastIcon = nullptr;
    QPushButton *m_toastAdvice = nullptr;
    GitAdviceKind m_toastKind = GitAdviceKind::Failure;
    QString m_toastError;
    GitAssistantController *m_assistant = nullptr;
    bool m_adviceDialogs = true;
    QString m_lastAdvice;
    bool m_mergeInProgress = false;
    int m_stashCount = 0;
    GitChangesView *m_changes = nullptr;
    GitCommitPlanView *m_plan = nullptr;
    GitHistoryView *m_history = nullptr;
    GitBranchesView *m_branches = nullptr;
};
