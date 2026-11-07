#include "git_panel.hpp"

#include "git_commit_plan_view.hpp"

#include "git_advice_dialog.hpp"
#include "git_assistant_bar.hpp"
#include "git_branches_view.hpp"
#include "git_guide.hpp"
#include "git_changes_view.hpp"
#include "git_history_view.hpp"
#include "git_repository.hpp"
#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QApplication>
#include <QStyle>
#include <QCheckBox>
#include <QMessageBox>
#include <QSettings>
#include <QHBoxLayout>
#include <QLabel>
#include <QMenu>
#include <QPushButton>
#include <QStackedWidget>
#include <QTabBar>
#include <QTabWidget>
#include <QToolButton>
#include <QVBoxLayout>

namespace {

constexpr int PollIntervalMs = 5000;
constexpr int DebounceMs = 250;
constexpr int ToastMs = 9000;
constexpr int ChangesTab = 0;
constexpr int HistoryTab = 1;
constexpr int BranchesTab = 2;

} // namespace

GitPanel::GitPanel(QString projectPath, QWidget *parent)
    : QWidget(parent), m_projectPath(std::move(projectPath))
{
    setObjectName(QStringLiteral("gitPanel"));
    m_repository = new GitRepository(m_projectPath, this);
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *header = new QWidget(this);
    auto *headerLayout = new QHBoxLayout(header);
    headerLayout->setContentsMargins(8, 6, 8, 2);
    m_branchButton = new QToolButton(header);
    m_branchButton->setPopupMode(QToolButton::InstantPopup);
    m_branchButton->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_branchButton->setIcon(studioIcon(QStringLiteral("studio-git"), QStyle::SP_FileIcon));
    m_branchButton->setIconSize(QSize(16, 16));
    m_branchButton->setStyleSheet(QStringLiteral("QToolButton { font-weight: 600; padding: 3px 8px; border-radius: 6px; } QToolButton::menu-indicator { image: none; }"));
    m_branchButton->setToolTip(QStringLiteral("Rama actual. Haz clic para cambiar de rama o crear una nueva."));
    m_branchButton->setMenu(new QMenu(m_branchButton));
    m_syncButton = new QToolButton(header);
    m_syncButton->setObjectName(QStringLiteral("gitSyncButton"));
    m_syncButton->setPopupMode(QToolButton::MenuButtonPopup);
    m_syncButton->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_syncButton->setIconSize(QSize(16, 16));
    m_syncButton->setStyleSheet(QStringLiteral("QToolButton#gitSyncButton { padding: 3px 20px 3px 8px; border-radius: 6px; }"));
    auto *syncMenu = new QMenu(m_syncButton);
    m_pullAction = syncMenu->addAction(QStringLiteral("Recibir (pull)"), this, [this] { pullNow(); });
    m_pushAction = syncMenu->addAction(QStringLiteral("Enviar (push)"), this, [this] { pushNow(); });
    m_fetchAction = syncMenu->addAction(QStringLiteral("Traer novedades sin aplicarlas (fetch)"), this, [this] { fetchNow(); });
    m_syncButton->setMenu(syncMenu);
    connect(m_syncButton, &QToolButton::clicked, this, [this] { syncNow(); });
    auto *reload = new QToolButton(header);
    reload->setIcon(studioIcon(QStringLiteral("view-refresh"), QStyle::SP_BrowserReload));
    reload->setIconSize(QSize(16, 16));
    reload->setToolTip(QStringLiteral("Actualizar"));
    reload->setAutoRaise(true);
    headerLayout->addWidget(m_branchButton);
    headerLayout->addStretch(1);
    headerLayout->addWidget(m_syncButton);
    headerLayout->addWidget(reload);
    layout->addWidget(header);

    m_stack = new QStackedWidget(this);
    auto *notRepository = new QWidget(m_stack);
    auto *notLayout = new QVBoxLayout(notRepository);
    m_notRepository = new QLabel(notRepository);
    m_notRepository->setAlignment(Qt::AlignCenter);
    m_notRepository->setWordWrap(true);
    m_init = new QPushButton(QStringLiteral("Iniciar control de versiones (git init)"), notRepository);
    notLayout->addStretch(1);
    notLayout->addWidget(m_notRepository);
    notLayout->addWidget(m_init, 0, Qt::AlignCenter);
    notLayout->addStretch(1);
    m_stack->addWidget(notRepository);

    m_tabs = new QTabWidget(m_stack);
    m_tabs->setDocumentMode(true);
    m_tabs->setUsesScrollButtons(false);
    m_tabs->tabBar()->setExpanding(true);
    m_tabs->tabBar()->setStyleSheet(QStringLiteral("QTabBar::tab { padding: 6px 6px; min-width: 0px; }"));
    m_changes = new GitChangesView(m_repository, m_tabs);
    m_history = new GitHistoryView(m_repository, m_tabs);
    m_branches = new GitBranchesView(m_repository, m_tabs);
    m_tabs->setIconSize(QSize(15, 15));
    m_tabs->addTab(m_changes, studioIcon(QStringLiteral("document-edit"), QStyle::SP_FileIcon), QStringLiteral("Cambios"));
    m_tabs->addTab(m_history, studioIcon(QStringLiteral("view-history"), QStyle::SP_FileIcon), QStringLiteral("Historial"));
    m_tabs->addTab(m_branches, studioIcon(QStringLiteral("studio-git"), QStyle::SP_FileIcon), QStringLiteral("Ramas"));
    m_stack->addWidget(m_tabs);
    layout->addWidget(m_stack, 1);

    m_toastBox = new QWidget(this);
    m_toastBox->setObjectName(QStringLiteral("gitToast"));
    auto *toastLayout = new QVBoxLayout(m_toastBox);
    toastLayout->setContentsMargins(0, 0, 0, 0);
    toastLayout->setSpacing(0);
    auto *toastRow = new QWidget(m_toastBox);
    auto *toastRowLayout = new QHBoxLayout(toastRow);
    toastRowLayout->setContentsMargins(10, 6, 10, 2);
    m_toastIcon = new QLabel(toastRow);
    m_toastIcon->setFixedSize(18, 18);
    m_toast = new QLabel(toastRow);
    m_toast->setWordWrap(true);
    m_toast->setTextInteractionFlags(Qt::TextSelectableByMouse);
    toastRowLayout->addWidget(m_toastIcon, 0, Qt::AlignTop);
    toastRowLayout->addWidget(m_toast, 1);
    m_toastAdvice = new QPushButton(studioIcon(QStringLiteral("studio-advice"), QStyle::SP_MessageBoxQuestion), QStringLiteral("Pedir consejo al asistente"), m_toastBox);
    m_toastAdvice->setFlat(true);
    toastLayout->addWidget(toastRow);
    toastLayout->addWidget(m_toastAdvice, 0, Qt::AlignLeft);
    m_toastBox->hide();
    layout->addWidget(m_toastBox);

    m_debounce.setSingleShot(true);
    m_debounce.setInterval(DebounceMs);
    m_toastTimer.setSingleShot(true);
    m_poll.setInterval(PollIntervalMs);
    connect(&m_debounce, &QTimer::timeout, this, [this] { refresh(); });
    connect(&m_poll, &QTimer::timeout, this, [this] {
        // Only while someone is looking: no background work for a hidden panel.
        if (isVisible() && window() != nullptr && window()->isActiveWindow()) refresh();
    });
    connect(&m_toastTimer, &QTimer::timeout, m_toastBox, &QWidget::hide);
    connect(m_toastAdvice, &QPushButton::clicked, this, [this] { adviseNow(m_toastKind, m_toastError); });
    connect(m_repository, &GitRepository::changed, this, [this] { m_historyStale = true; m_branchesStale = true; scheduleRefresh(); });
    connect(reload, &QToolButton::clicked, this, [this] { m_historyStale = true; m_branchesStale = true; refresh(); });
    connect(m_init, &QPushButton::clicked, this, [this] {
        m_repository->init([this](const GitResult &result) {
            if (!result.ok()) showToast(result.message(), true);
            else { m_detected = false; refresh(); }
        });
    });
    connect(m_tabs, &QTabWidget::currentChanged, this, [this](int index) { tabChanged(index); });
    for (GitChangesView *view : {m_changes}) {
        connect(view, &GitChangesView::openDiffRequested, this, &GitPanel::openDiffRequested);
        connect(view, &GitChangesView::openFileRequested, this, &GitPanel::openFileRequested);
        connect(view, &GitChangesView::message, this, [this](const QString &text, bool error) { showToast(text, error); });
    }
    connect(m_history, &GitHistoryView::openDiffRequested, this, &GitPanel::openDiffRequested);
    connect(m_history, &GitHistoryView::message, this, [this](const QString &text, bool error) { showToast(text, error); });
    connect(m_history, &GitHistoryView::createBranchRequested, this, [this](const QString &hash) {
        m_tabs->setCurrentIndex(BranchesTab);
        m_branches->createNamed(QStringLiteral("rama-%1").arg(hash.left(7)), hash);
    });
    connect(m_branches, &GitBranchesView::message, this, [this](const QString &text, bool error) { showToast(text, error); });
    connect(m_branchButton->menu(), &QMenu::aboutToShow, this, [this] { rebuildBranchMenu(); });
    m_assistant = new GitAssistantController(m_repository, m_projectPath, this);
    connect(m_assistant, &GitAssistantController::agentsChanged, this, [this] { updateGuidance(); });
    connect(m_assistant, &GitAssistantController::modelsChanged, this, [this] { updateGuidance(); });
    connect(m_assistant, &GitAssistantController::busyChanged, this, [this](bool busy, const QString &text) { m_changes->setAssistantBusy(busy, text); });
    connect(m_assistant, &GitAssistantController::failed, this, [this](const QString &message) { showToast(message, true, GitProblem::Other, false); });
    connect(m_assistant, &GitAssistantController::commitMessageReady, this, [this](const QString &message, const QString &agent) {
        m_tabs->setCurrentIndex(ChangesTab);
        m_changes->setMessage(message);
        m_changes->setAssistantNote(QStringLiteral("Sugerido por %1. Revísalo y edítalo antes de guardar.").arg(agent));
        showToast(QStringLiteral("Mensaje sugerido. Puedes editarlo antes de guardar."), false);
    });
    m_plan = new GitCommitPlanView(this);
    m_changes->insertPlanView(m_plan);
    connect(m_assistant, &GitAssistantController::commitPlanReady, this, [this](const QList<GitCommitGroup> &plan, const QString &agent) {
        m_tabs->setCurrentIndex(ChangesTab);
        m_plan->setPlan(plan, agent);
        preparePlanGroup(0);
        showToast(QStringLiteral("Agrupé los cambios en %1 commits. Revisa el primero y guárdalo; luego prepara el siguiente.").arg(plan.size()), false);
    });
    connect(m_plan, &GitCommitPlanView::prepareRequested, this, [this](int index) { preparePlanGroup(index); });
    connect(m_plan, &GitCommitPlanView::dismissed, this, [this] { m_plan->setPlan({}, QString()); });
    connect(m_assistant, &GitAssistantController::adviceReady, this, [this](GitAdviceKind kind, const QString &markdown, const QString &agent) {
        m_lastAdvice = markdown;
        if (!m_adviceDialogs) return;
        GitAdviceDialog dialog(adviceTitle(kind), markdown, agent, this);
        dialog.exec();
    });
    connect(m_changes, &GitChangesView::suggestMessageRequested, m_assistant, &GitAssistantController::suggestCommitMessage);
    connect(m_changes, &GitChangesView::adviceRequested, this, [this](GitAdviceKind kind) { adviseNow(kind); });
    connect(m_changes, &GitChangesView::assistantCancelRequested, m_assistant, &GitAssistantController::cancel);
    connect(m_changes, &GitChangesView::assistantAgentChosen, m_assistant, &GitAssistantController::chooseAgent);
    connect(m_changes, &GitChangesView::assistantModelChosen, m_assistant, &GitAssistantController::setModelPreference);
    m_poll.start();
    showNotRepository(false);
    m_notRepository->setText(QStringLiteral("Buscando el repositorio…"));
    m_init->hide();
    refresh();
}

GitPanel::~GitPanel() = default;

// Makes the commit be the group's files and nothing else from the plan, and puts its message in the box.
void GitPanel::preparePlanGroup(int index)
{
    if (index < 0 || index >= m_plan->groupCount()) return;
    const GitCommitGroup group = m_plan->plan().at(index);
    QStringList planned;
    for (const GitCommitGroup &other : m_plan->plan()) planned << other.files;
    QStringList staged;
    for (const GitEntry &entry : m_status.stagedEntries()) staged << entry.path;
    QStringList outside;
    for (const QString &path : staged) if (planned.contains(path) && !group.files.contains(path)) outside << path;
    const auto stageGroup = [this, group] {
        m_repository->stage(group.files, [this, group](const GitResult &result) {
            if (!result.ok()) return showToast(QStringLiteral("No pude preparar los archivos de este commit."), true);
            m_changes->setMessage(group.message);
            m_changes->setAssistantNote(QStringLiteral("Commit preparado con %1 archivo(s). Revísalo y guárdalo.").arg(group.files.size()));
            refresh();
        });
    };
    if (outside.isEmpty()) return stageGroup();
    m_repository->unstage(outside, m_status.initial, [this, stageGroup](const GitResult &result) {
        if (!result.ok()) return showToast(QStringLiteral("No pude separar los archivos de otros commits."), true);
        stageGroup();
    });
}

void GitPanel::scheduleRefresh()
{
    m_debounce.start();
}

void GitPanel::refresh()
{
    if (m_refreshing) {
        m_dirty = true;
        return;
    }
    m_refreshing = true;
    const auto readStatus = [this] {
        m_repository->status([this](const GitResult &result, const GitStatus &status) {
            if (!result.ok()) {
                m_refreshing = false;
                return showNotRepository(result.startFailed);
            }
            applyStatus(status);
            m_refreshing = false;
            if (m_dirty) {
                m_dirty = false;
                scheduleRefresh();
            }
        });
    };
    if (m_detected) return readStatus();
    m_repository->detect([this, readStatus](bool isRepository) {
        m_detected = true;
        if (!isRepository) {
            m_refreshing = false;
            return showNotRepository(false);
        }
        readStatus();
    });
}

void GitPanel::showNotRepository(bool gitMissing)
{
    const bool was = m_isRepository;
    m_isRepository = false;
    m_changeCount = 0;
    m_detected = false;
    m_stack->setCurrentIndex(0);
    m_branchButton->setText(QStringLiteral("Git"));
    m_branchButton->setEnabled(false);
    m_syncButton->hide();
    emit explorerStatusChanged(GitExplorerStatus());
    if (gitMissing) {
        m_notRepository->setText(QStringLiteral("No encontré Git en este equipo.\nInstálalo desde https://git-scm.com y reinicia Studio."));
        m_init->hide();
    } else {
        m_notRepository->setText(QStringLiteral("Esta carpeta todavía no usa control de versiones.\n\nCon Git puedes guardar versiones de tu proyecto, volver atrás y trabajar en ramas."));
        m_init->show();
    }
    if (was) emit changeCountChanged(0);
}

void GitPanel::applyStatus(const GitStatus &status)
{
    const bool becameRepository = !m_isRepository;
    m_isRepository = true;
    m_status = status;
    m_stack->setCurrentIndex(1);
    m_branchButton->setEnabled(true);
    m_branchButton->setText(status.detached ? QStringLiteral("(sin rama)") : (status.head.isEmpty() ? QStringLiteral("main") : status.head));
    m_syncButton->show();
    m_changes->setStatus(status);
    if (m_plan != nullptr && m_plan->groupCount() > 0) {
        // Committed or edited files drop out of the plan; what is left stays proposed.
        QStringList pending;
        for (const GitEntry &entry : status.stagedEntries()) pending << entry.path;
        for (const GitEntry &entry : status.changedEntries()) pending << entry.path;
        m_plan->setPlan(reconcileCommitPlan(m_plan->plan(), pending), QString());
    }
    emit explorerStatusChanged(buildExplorerStatus(status, m_repository->root()));
    m_repository->stashList([this](const GitResult &result, const QList<GitStash> &stashes) {
        if (!result.ok()) return;
        m_stashCount = int(stashes.size());
        m_changes->setStashes(stashes);
        updateGuidance();
    });
    if (!status.conflicts().isEmpty()) {
        m_repository->mergeInProgress([this](bool inProgress) {
            m_mergeInProgress = inProgress;
            m_changes->setMergeInProgress(inProgress);
            updateGuidance();
        });
    } else {
        m_mergeInProgress = false;
        m_changes->setMergeInProgress(false);
    }
    if (becameRepository || m_remotes.isEmpty()) {
        m_repository->remotes([this](const GitResult &, const QStringList &names) {
            m_remotes = names;
            updateSyncButton();
            updateGuidance();
        });
    }
    updateSyncButton();
    updateGuidance();
    const int count = int(status.entries.size());
    const QString head = status.oid + status.head;
    if (head != m_lastHead || becameRepository) {
        m_lastHead = head;
        m_historyStale = true;
        m_branchesStale = true;
    }
    if (count != m_changeCount) {
        m_changeCount = count;
        emit changeCountChanged(count);
    }
    if (m_branchesStale) loadBranches();
    if (m_historyStale && (m_tabs->currentIndex() == HistoryTab || becameRepository)) {
        m_historyStale = false;
        m_history->reload();
    }
}

void GitPanel::loadBranches()
{
    m_branchesStale = false;
    m_repository->branches([this](const GitResult &result, const QList<GitBranch> &branches) {
        if (!result.ok()) return;
        m_branchList = branches;
        m_branches->setBranches(branches);
    });
}

void GitPanel::tabChanged(int index)
{
    if (index == HistoryTab && m_historyStale && m_isRepository) {
        m_historyStale = false;
        m_history->reload();
    }
    if (index == BranchesTab && m_branchesStale && m_isRepository) loadBranches();
}

void GitPanel::showTab(int index)
{
    m_tabs->setCurrentIndex(index);
}

void GitPanel::rebuildBranchMenu()
{
    QMenu *menu = m_branchButton->menu();
    menu->clear();
    int shown = 0;
    for (const GitBranch &branch : m_branchList) {
        if (branch.remote || shown >= 12) continue;
        QAction *action = menu->addAction((branch.current ? QStringLiteral("✓ ") : QStringLiteral("    ")) + branch.name);
        action->setEnabled(!branch.current);
        connect(action, &QAction::triggered, this, [this, name = branch.name] { m_branches->switchTo(name, false); });
        ++shown;
    }
    menu->addSeparator();
    menu->addAction(QStringLiteral("Ver todas las ramas…"), this, [this] { m_tabs->setCurrentIndex(BranchesTab); });
}

void GitPanel::showToast(const QString &text, bool error, GitProblem problem, bool offerAdvice)
{
    const StudioTheme &theme = ThemeManager::current();
    m_toast->setText(text);
    const QColor tone = error ? theme.danger : theme.success;
    m_toastBox->setStyleSheet(QStringLiteral("#gitToast { border-top: 1px solid %1; }").arg(tone.name()));
    m_toast->setStyleSheet(QStringLiteral("QLabel { color: %1; }").arg(tone.name()));
    m_toastIcon->setPixmap(renderStudioIcon(error ? QStringLiteral("dialog-warning") : QStringLiteral("check-circle"), tone, 16));
    // An error can be explained by the assistant: what the problem was decides what it is asked.
    m_toastError = text;
    switch (problem) {
    case GitProblem::Rejected: m_toastKind = GitAdviceKind::PushRejected; break;
    case GitProblem::Credentials: m_toastKind = GitAdviceKind::Credentials; break;
    case GitProblem::Diverged: m_toastKind = GitAdviceKind::Diverged; break;
    default: m_toastKind = GitAdviceKind::Failure; break;
    }
    m_toastAdvice->setVisible(error && offerAdvice);
    m_toastBox->show();
    if (error) m_toastTimer.stop();
    else m_toastTimer.start(ToastMs);
}

QString GitPanel::toastText() const { return m_toastBox->isVisibleTo(this) ? m_toast->text() : QString(); }
bool GitPanel::toastOffersAdvice() const { return m_toastBox->isVisibleTo(this) && m_toastAdvice->isVisibleTo(m_toastBox); }
QString GitPanel::branchText() const { return m_branchButton->text(); }
QString GitPanel::syncText() const { return m_syncButton->text(); }
QString GitPanel::stateText() const { return m_stack->currentIndex() == 0 ? m_notRepository->text() : QString(); }

QString GitPanel::remoteName() const
{
    if (!m_status.upstream.isEmpty()) return m_status.upstream.section(QLatin1Char('/'), 0, 0);
    return m_remotes.contains(QStringLiteral("origin")) ? QStringLiteral("origin") : m_remotes.value(0);
}

void GitPanel::updateSyncButton()
{
    if (!m_isRepository) return;
    if (m_syncing) {
        m_syncButton->setText(QStringLiteral("Cancelar"));
        m_syncButton->setIcon(studioIcon(QStringLiteral("close-small"), QStyle::SP_DialogCancelButton));
        m_syncButton->setToolTip(QStringLiteral("Detiene la operación en curso. No se pierde nada de lo ya guardado."));
        return;
    }
    const bool hasRemote = !m_remotes.isEmpty();
    const bool hasBranch = !m_status.detached;
    QString text;
    QString tip;
    QString icon = QStringLiteral("view-refresh");
    if (!hasRemote) {
        text = QStringLiteral("Sin remoto");
        icon = QStringLiteral("dialog-warning");
        tip = QStringLiteral("Este repositorio no tiene un remoto (por ejemplo origin), así que no hay con quién sincronizar.\nAñade uno con: git remote add origin <url>");
    } else if (!hasBranch) {
        text = QStringLiteral("Sin rama");
        icon = QStringLiteral("dialog-warning");
        tip = QStringLiteral("Estás sobre un commit suelto, no sobre una rama: crea una rama para poder enviar.");
    } else if (m_status.behind > 0) {
        text = QStringLiteral("Recibir %1").arg(m_status.behind);
        icon = QStringLiteral("git-pull");
        tip = QStringLiteral("%1 commit(s) nuevos en %2. Haz clic para recibirlos (pull).").arg(m_status.behind).arg(m_status.upstream);
        if (m_status.ahead > 0) tip += QStringLiteral("\nTambién tienes %1 por enviar: recibe primero.").arg(m_status.ahead);
    } else if (m_status.ahead > 0) {
        text = QStringLiteral("Enviar %1").arg(m_status.ahead);
        icon = QStringLiteral("git-push");
        tip = QStringLiteral("%1 commit(s) por enviar a %2. Haz clic para enviarlos (push).").arg(m_status.ahead).arg(m_status.upstream);
    } else if (m_status.upstream.isEmpty()) {
        text = QStringLiteral("Publicar rama");
        icon = QStringLiteral("git-push");
        tip = QStringLiteral("Esta rama solo existe en tu equipo. Haz clic para publicarla en %1 (push).").arg(remoteName());
    } else {
        text = QStringLiteral("Traer novedades");
        icon = QStringLiteral("view-refresh");
        tip = QStringLiteral("Estás al día con %1. Haz clic para comprobar si hay novedades (fetch).").arg(m_status.upstream);
    }
    m_syncButton->setText(text);
    m_syncButton->setIcon(studioIcon(icon, QStyle::SP_BrowserReload));
    m_syncButton->setToolTip(tip);
    m_syncButton->setEnabled(hasRemote && hasBranch);
    m_pullAction->setEnabled(hasRemote && !m_status.upstream.isEmpty());
    m_pushAction->setEnabled(hasRemote && hasBranch);
    m_fetchAction->setEnabled(hasRemote);
}

void GitPanel::beginSync(const QString &text)
{
    m_syncing = true;
    updateSyncButton();
    m_syncButton->setEnabled(true);
    showToast(text, false);
    m_toastTimer.stop();
}

void GitPanel::endSync()
{
    m_syncing = false;
    updateSyncButton();
}

void GitPanel::cancelSync()
{
    if (m_syncing) m_repository->cancelCurrent();
}

void GitPanel::syncNow()
{
    if (m_syncing) return cancelSync();
    if (m_remotes.isEmpty() || m_status.detached) return;
    if (m_status.behind > 0) pullNow();
    else if (m_status.ahead > 0 || m_status.upstream.isEmpty()) pushNow();
    else fetchNow();
}

void GitPanel::fetchNow()
{
    if (m_syncing || m_remotes.isEmpty()) return;
    beginSync(QStringLiteral("Buscando novedades…"));
    m_repository->fetch([this](const GitResult &result) {
        endSync();
        if (result.problem() == GitProblem::Cancelled) return showToast(QStringLiteral("Cancelado."), false);
        showToast(result.ok() ? QStringLiteral("Novedades traídas. Revisa si hay commits por recibir.") : result.message(), !result.ok(), result.problem());
    });
}

void GitPanel::pullNow(GitPullMode mode)
{
    if (m_syncing || m_remotes.isEmpty()) return;
    beginSync(QStringLiteral("Recibiendo cambios…"));
    m_repository->pull(mode, [this](const GitResult &result) {
        endSync();
        if (result.problem() == GitProblem::Cancelled) return showToast(QStringLiteral("Cancelado."), false);
        if (result.problem() == GitProblem::Diverged && m_confirm) {
            QMessageBox box(this);
            box.setWindowTitle(QStringLiteral("Tu rama y la del remoto se separaron"));
            box.setIcon(QMessageBox::Question);
            box.setText(QStringLiteral("Hay commits nuevos en el remoto y también commits tuyos que aún no envías.\n\n"
                                       "Puedes fusionarlos (merge): Git crea un commit que une los dos lados. Si hay conflictos, te los mostraré y podrás cancelar la fusión."));
            QPushButton *merge = box.addButton(QStringLiteral("Fusionar (merge)"), QMessageBox::AcceptRole);
            box.addButton(QMessageBox::Cancel);
            box.setDefaultButton(QMessageBox::Cancel);
            box.exec();
            if (box.clickedButton() == merge) return pullNow(GitPullMode::Merge);
            return showToast(QStringLiteral("No se recibió nada. Tu rama sigue como estaba."), false);
        }
        if (!result.ok()) return showToast(result.message(), true, result.problem());
        showToast(QString::fromUtf8(result.out).contains(QLatin1String("Already up to date")) ? QStringLiteral("Ya estabas al día.")
                                                                                              : QStringLiteral("Cambios recibidos."), false);
    });
}

bool GitPanel::confirmPush(const QString &text)
{
    if (!m_confirm || !QSettings().value(QStringLiteral("git/confirmPush"), true).toBool()) return true;
    QMessageBox box(this);
    box.setWindowTitle(QStringLiteral("Enviar al remoto"));
    box.setIcon(QMessageBox::Question);
    box.setText(text);
    auto *again = new QCheckBox(QStringLiteral("No volver a preguntar"), &box);
    box.setCheckBox(again);
    box.setStandardButtons(QMessageBox::Ok | QMessageBox::Cancel);
    box.setDefaultButton(QMessageBox::Ok);
    const bool accepted = box.exec() == QMessageBox::Ok;
    if (accepted && again->isChecked()) QSettings().setValue(QStringLiteral("git/confirmPush"), false);
    return accepted;
}

void GitPanel::pushNow()
{
    if (m_syncing || m_remotes.isEmpty() || m_status.detached) return;
    const QString remote = remoteName();
    const QString branch = m_status.head;
    const bool publish = m_status.upstream.isEmpty();
    const QString question = publish
        ? QStringLiteral("Se publicará la rama «%1» en %2 y quedará siguiendo a %2/%1. Cualquiera con acceso al remoto podrá verla.").arg(branch, remote)
        : QStringLiteral("Se enviarán %1 commit(s) a %2.").arg(m_status.ahead).arg(m_status.upstream);
    if (!confirmPush(question)) return;
    beginSync(publish ? QStringLiteral("Publicando la rama…") : QStringLiteral("Enviando cambios…"));
    m_repository->push(remote, branch, publish, [this, remote, branch, publish](const GitResult &result) {
        endSync();
        if (result.problem() == GitProblem::Cancelled) return showToast(QStringLiteral("Cancelado."), false);
        if (!result.ok()) return showToast(result.message(), true, result.problem());
        showToast(publish ? QStringLiteral("Rama «%1» publicada en %2.").arg(branch, remote) : QStringLiteral("Cambios enviados."), false);
    });
}

void GitPanel::updateGuidance()
{
    if (!m_isRepository) return;
    GitGuideFacts facts;
    facts.mergeInProgress = m_mergeInProgress;
    facts.hasRemotes = !m_remotes.isEmpty();
    facts.stashCount = m_stashCount;
    m_assistant->setSnapshot(m_status, m_mergeInProgress, m_stashCount);
    m_changes->setGuidance(guidanceFor(m_status, facts));
    m_changes->setAssistantAgents(m_assistant->agents(), m_assistant->currentAgentId());
    const QString agent = m_assistant->currentAgentId();
    m_changes->setAssistantModels(m_assistant->modelList(agent), m_assistant->modelPreference(agent));
    m_assistant->refreshModels();
}

void GitPanel::configureAssistant(const QString &cliProgram, GitAssistantController::AgentProvider agents, GitAssistantController::PreferredProvider preferred)
{
    m_assistant->configure(cliProgram, std::move(agents), std::move(preferred));
    updateGuidance();
}

void GitPanel::adviseNow(GitAdviceKind kind, const QString &lastError)
{
    // With conflicts on the table the useful question is always the conflicts one.
    const GitAdviceKind effective = !m_status.conflicts().isEmpty() ? GitAdviceKind::Conflicts : kind;
    m_assistant->advise(effective, lastError);
}
