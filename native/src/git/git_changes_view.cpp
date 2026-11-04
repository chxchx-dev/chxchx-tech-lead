#include "git_changes_view.hpp"

#include "git_assistant_bar.hpp"
#include "git_change_delegate.hpp"
#include "git_graph_delegate.hpp"
#include "git_parsers.hpp"
#include "git_repository.hpp"
#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QDialog>
#include <QDialogButtonBox>
#include <QLineEdit>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMenu>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollBar>
#include <QShortcut>
#include <QToolButton>
#include <QTreeWidget>
#include <QVBoxLayout>

namespace {

QString letterOf(const GitEntry &entry, const QString &section)
{
    if (entry.conflicted()) return QStringLiteral("U");
    if (entry.untracked()) return QStringLiteral("?");
    return QString(section == QLatin1String("staged") ? entry.index : entry.work);
}

QStringList pathsOf(const QList<GitEntry> &entries)
{
    QStringList paths;
    for (const GitEntry &entry : entries) paths << entry.path;
    return paths;
}

} // namespace

GitChangesView::GitChangesView(GitRepository *repository, QWidget *parent)
    : QWidget(parent), m_repository(repository)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(6);

    m_bar = new GitAssistantBar(this);
    m_note = new QLabel(this);
    m_note->setWordWrap(true);
    m_note->hide();
    m_message = new QPlainTextEdit(this);
    m_message->setPlaceholderText(QStringLiteral("Describe qué cambiaste. Ejemplo: Corrige el error al guardar"));
    m_message->setMinimumHeight(66);
    m_message->setMaximumHeight(66);
    m_message->setTabChangesFocus(true);
    m_hint = new QLabel(this);
    m_hint->setWordWrap(true);
    m_commit = new QPushButton(this);
    m_commit->setObjectName(QStringLiteral("gitCommitButton"));
    m_commit->setCursor(Qt::PointingHandCursor);
    m_banner = new QLabel(this);
    m_banner->setWordWrap(true);
    m_banner->hide();
    m_abortMerge = new QPushButton(QStringLiteral("Cancelar la fusión (merge --abort)"), this);
    m_abortMerge->setToolTip(QStringLiteral("Vuelve a como estaba antes de recibir los cambios. No pierdes lo que ya habías guardado."));
    m_abortMerge->hide();
    m_emptyBox = new QWidget(this);
    auto *emptyLayout = new QVBoxLayout(m_emptyBox);
    emptyLayout->setAlignment(Qt::AlignCenter);
    m_emptyIcon = new QLabel(m_emptyBox);
    m_emptyIcon->setAlignment(Qt::AlignCenter);
    m_empty = new QLabel(QStringLiteral("Todo está guardado.\nNo hay cambios pendientes."), m_emptyBox);
    m_empty->setAlignment(Qt::AlignCenter);
    emptyLayout->addWidget(m_emptyIcon);
    emptyLayout->addWidget(m_empty);
    m_emptyBox->hide();
    m_tree = new QTreeWidget(this);
    m_tree->setHeaderHidden(true);
    m_tree->setFrameShape(QFrame::NoFrame);
    m_tree->setIndentation(0);
    m_tree->setMouseTracking(true);
    m_tree->setSelectionMode(QAbstractItemView::ExtendedSelection);
    m_tree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_tree->setItemDelegate(new GitChangeDelegate([this](const QModelIndex &index, GitChangeDelegate::Action action) {
        const QString path = index.data(GitChangeRole::Path).toString();
        const QString section = index.data(GitChangeRole::Section).toString();
        if (action == GitChangeDelegate::Action::Stage) stagePaths({path});
        else if (action == GitChangeDelegate::Action::Unstage) unstagePaths({path});
        else if (action == GitChangeDelegate::Action::Discard) discardPaths({path});
        else if (action == GitChangeDelegate::Action::Apply) applyStash(path.toInt());
        else if (action == GitChangeDelegate::Action::Drop) dropStash(path.toInt());
        Q_UNUSED(section);
    }, this));
    m_suggest = new QToolButton(this);
    m_suggest->setObjectName(QStringLiteral("gitSuggestButton"));
    m_suggest->setText(QStringLiteral("Sugerir"));
    m_suggest->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_suggest->setIcon(studioIcon(QStringLiteral("studio-assistant"), QStyle::SP_MessageBoxInformation));
    m_suggest->setToolTip(QStringLiteral("Pide al asistente que redacte el mensaje a partir de tus cambios. Podrás editarlo antes de guardar."));
    auto *commitRow = new QHBoxLayout;
    commitRow->setSpacing(6);
    commitRow->addWidget(m_suggest);
    commitRow->addWidget(m_commit, 1);
    layout->addWidget(m_bar);
    layout->addWidget(m_message);
    layout->addWidget(m_note);
    layout->addLayout(commitRow);
    layout->addWidget(m_hint);
    layout->addWidget(m_banner);
    layout->addWidget(m_abortMerge);
    layout->addWidget(m_tree, 1);
    layout->addWidget(m_emptyBox, 1);

    connect(m_message, &QPlainTextEdit::textChanged, this, [this] {
        // The box grows with the message (a suggested one has a body) up to a limit, then scrolls.
        // A plain text document measures its size in lines, not pixels.
        m_message->document()->setTextWidth(qMax(60, m_message->viewport()->width()));
        const int lines = qMax(1, int(m_message->document()->size().height()));
        const int wanted = qBound(66, lines * m_message->fontMetrics().lineSpacing() + 38, 200);
        m_message->setMinimumHeight(wanted);
        m_message->setMaximumHeight(wanted);
        if (m_message->toPlainText().isEmpty()) m_note->hide();
        updateCommitArea();
    });
    connect(m_commit, &QPushButton::clicked, this, [this] { commitNow(); });
    connect(m_abortMerge, &QPushButton::clicked, this, [this] { abortMerge(); });
    connect(m_suggest, &QToolButton::clicked, this, [this] { emit suggestMessageRequested(); });
    connect(m_bar, &GitAssistantBar::adviceRequested, this, &GitChangesView::adviceRequested);
    connect(m_bar, &GitAssistantBar::cancelRequested, this, &GitChangesView::assistantCancelRequested);
    connect(m_bar, &GitAssistantBar::agentChosen, this, &GitChangesView::assistantAgentChosen);
    connect(m_bar, &GitAssistantBar::modelChosen, this, &GitChangesView::assistantModelChosen);
    auto *shortcut = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Return")), m_message, nullptr, nullptr, Qt::WidgetShortcut);
    connect(shortcut, &QShortcut::activated, this, [this] { commitNow(); });
    connect(m_tree, &QTreeWidget::itemClicked, this, [this](QTreeWidgetItem *item) {
        if (item->parent() != nullptr) openDiffFor(m_tree->indexFromItem(item));
    });
    connect(m_tree, &QTreeWidget::customContextMenuRequested, this, [this](const QPoint &position) { contextMenu(position); });
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, [this] { restyle(); rebuild(); });
    restyle();
    updateCommitArea();
}

void GitChangesView::restyle()
{
    const StudioTheme &theme = ThemeManager::current();
    // The commit button is the primary action: it takes the accent colour, and greys out when it cannot be used.
    m_commit->setStyleSheet(QStringLiteral(
        "#gitCommitButton { background-color: %1; color: %2; border: none; border-radius: 6px; padding: 8px 12px; font-weight: 600; }"
        "#gitCommitButton:hover { background-color: %3; }"
        "#gitCommitButton:disabled { background-color: %4; color: %5; }")
        .arg(theme.accent.name(), theme.onAccent.name(), theme.accentStrong.name(), theme.surfaceRaised.name(), theme.textFaint.name()));
    QIcon icon;
    icon.addPixmap(renderStudioIcon(QStringLiteral("git-commit"), theme.onAccent, 18), QIcon::Normal);
    icon.addPixmap(renderStudioIcon(QStringLiteral("git-commit"), theme.textFaint, 18), QIcon::Disabled);
    m_commit->setIcon(icon);
    m_commit->setIconSize(QSize(18, 18));
    m_message->setStyleSheet(QStringLiteral(
        "QPlainTextEdit { background-color: %1; border: 1px solid %2; border-radius: 6px; padding: 6px; }"
        "QPlainTextEdit:focus { border: 1px solid %3; }").arg(theme.surface.name(), theme.border.name(), theme.accent.name()));
    m_suggest->setStyleSheet(QStringLiteral(
        "#gitSuggestButton { border: 1px solid %1; border-radius: 6px; padding: 6px 10px; color: %2; }"
        "#gitSuggestButton:hover { background-color: %3; }"
        "#gitSuggestButton:disabled { border-color: %4; color: %5; }")
        .arg(theme.accent.name(), theme.accent.name(), theme.accentSoft.name(), theme.border.name(), theme.textFaint.name()));
    m_emptyIcon->setPixmap(renderStudioIcon(QStringLiteral("check-circle"), theme.success, 44));
    QPalette palette = m_empty->palette();
    palette.setColor(QPalette::WindowText, theme.textMuted);
    m_empty->setPalette(palette);
    m_note->setStyleSheet(QStringLiteral("QLabel { color: %1; }").arg(theme.accent.name()));
    m_hint->setStyleSheet(QStringLiteral("QLabel { color: %1; }").arg(theme.textMuted.name()));
}

void GitChangesView::setStatus(const GitStatus &status)
{
    m_status = status;
    rebuild();
    updateCommitArea();
}

QTreeWidgetItem *GitChangesView::addSection(const QString &key, const QString &title, int count)
{
    auto *item = new QTreeWidgetItem(m_tree);
    item->setData(0, GitChangeRole::Section, key);
    item->setFlags(Qt::ItemIsEnabled);
    item->setSizeHint(0, QSize(0, 32));
    m_tree->setFirstColumnSpanned(m_tree->indexOfTopLevelItem(item), QModelIndex(), true);
    m_tree->setItemWidget(item, 0, sectionHeader(key, title, count));
    item->setExpanded(true);
    return item;
}

QWidget *GitChangesView::sectionHeader(const QString &key, const QString &title, int count)
{
    auto *header = new QWidget;
    auto *row = new QHBoxLayout(header);
    row->setContentsMargins(8, 2, 6, 2);
    const StudioTheme &theme = ThemeManager::current();
    QColor accent = theme.textMuted;
    QString icon = QStringLiteral("document-edit");
    if (key == QLatin1String("conflicts")) { accent = theme.warning; icon = QStringLiteral("dialog-warning"); }
    else if (key == QLatin1String("staged")) { accent = theme.success; icon = QStringLiteral("git-commit"); }
    else if (key == QLatin1String("stashes")) { accent = theme.accent; icon = QStringLiteral("git-stash"); }
    auto *iconLabel = new QLabel(header);
    iconLabel->setPixmap(renderStudioIcon(icon, accent, 16));
    iconLabel->setFixedSize(18, 18);
    row->addWidget(iconLabel);
    auto *label = new QLabel(QStringLiteral("<b>%1</b> <span style='color:%3'>· %2</span>").arg(title.toHtmlEscaped()).arg(count).arg(theme.textMuted.name()), header);
    row->addWidget(label, 1);
    const auto button = [&](const QString &iconName, const QString &text, const QString &tip) {
        auto *tool = new QToolButton(header);
        tool->setIcon(studioIcon(iconName, QStyle::SP_FileIcon));
        tool->setIconSize(QSize(16, 16));
        tool->setText(text);
        tool->setToolButtonStyle(text.isEmpty() ? Qt::ToolButtonIconOnly : Qt::ToolButtonTextBesideIcon);
        tool->setToolTip(tip);
        tool->setAutoRaise(true);
        row->addWidget(tool);
        return tool;
    };
    if (key == QLatin1String("staged")) {
        connect(button(QStringLiteral("list-remove"), QString(), QStringLiteral("Quitar todos de los preparados (unstage all)")), &QToolButton::clicked, this,
            [this] { unstagePaths(pathsOf(m_status.stagedEntries())); });
    } else if (key == QLatin1String("changes")) {
        connect(button(QStringLiteral("git-stash"), QString(), QStringLiteral("Apartar los cambios (stash): los guarda aparte y deja el proyecto limpio, para recuperarlos después")),
            &QToolButton::clicked, this, [this] { askStash(); });
        connect(button(QStringLiteral("edit-undo"), QString(), QStringLiteral("Descartar todos los cambios (se guarda una copia de seguridad)")), &QToolButton::clicked, this,
            [this] { discardPaths(pathsOf(m_status.changedEntries())); });
        connect(button(QStringLiteral("list-add"), QString(), QStringLiteral("Preparar todos para guardar (stage all)")), &QToolButton::clicked, this,
            [this] { stagePaths(pathsOf(m_status.changedEntries())); });
    }
    return header;
}

void GitChangesView::rebuild()
{
    const int scroll = m_tree->verticalScrollBar()->value();
    QSet<QString> selected;
    for (QTreeWidgetItem *item : m_tree->selectedItems()) {
        selected.insert(item->data(0, GitChangeRole::Section).toString() + QLatin1Char('\n') + item->data(0, GitChangeRole::Path).toString());
    }
    m_tree->clear();
    const QList<GitEntry> conflicts = m_status.conflicts();
    const QList<GitEntry> staged = m_status.stagedEntries();
    const QList<GitEntry> changes = m_status.changedEntries();
    const bool nothing = conflicts.isEmpty() && staged.isEmpty() && changes.isEmpty() && m_stashes.isEmpty();
    m_tree->setVisible(!nothing);
    m_emptyBox->setVisible(nothing);
    m_banner->setVisible(!conflicts.isEmpty());
    m_abortMerge->setVisible(!conflicts.isEmpty() && m_mergeInProgress);
    if (!conflicts.isEmpty()) {
        m_banner->setText(QStringLiteral("Hay conflictos: abre cada archivo, resuelve las marcas <<<<<<< ======= >>>>>>> y prepáralo con + para poder guardar."));
    }
    const auto fill = [&](const QString &key, const QString &title, const QList<GitEntry> &entries) {
        if (entries.isEmpty()) return;
        QTreeWidgetItem *section = addSection(key, title, int(entries.size()));
        for (const GitEntry &entry : entries) {
            auto *item = new QTreeWidgetItem(section);
            item->setData(0, GitChangeRole::Path, entry.path);
            item->setData(0, GitChangeRole::Section, key);
            item->setData(0, GitChangeRole::Letter, letterOf(entry, key));
            item->setData(0, GitChangeRole::Untracked, entry.untracked());
            const QString label = changeLabel(key == QLatin1String("staged") ? entry.index : entry.work, entry.untracked());
            item->setToolTip(0, QStringLiteral("%1 · %2%3\nClic: ver los cambios").arg(label, entry.path,
                entry.origPath.isEmpty() ? QString() : QStringLiteral("\n(antes: %1)").arg(entry.origPath)));
            if (selected.contains(key + QLatin1Char('\n') + entry.path)) item->setSelected(true);
        }
        section->setExpanded(true);
    };
    fill(QStringLiteral("conflicts"), QStringLiteral("Conflictos"), conflicts);
    fill(QStringLiteral("staged"), QStringLiteral("Listos para guardar (staged)"), staged);
    fill(QStringLiteral("changes"), QStringLiteral("Cambios"), changes);
    if (!m_stashes.isEmpty()) {
        QTreeWidgetItem *section = addSection(QStringLiteral("stashes"), QStringLiteral("Apartados (stash)"), int(m_stashes.size()));
        for (const GitStash &stash : m_stashes) {
            auto *item = new QTreeWidgetItem(section);
            item->setData(0, GitChangeRole::Path, QString::number(stash.index));
            item->setData(0, GitChangeRole::Section, QStringLiteral("stashes"));
            item->setData(0, GitChangeRole::Letter, QStringLiteral("S"));
            item->setData(0, GitChangeRole::Title, stash.message);
            item->setData(0, GitChangeRole::Detail, gitRelativeTime(stash.time));
            item->setToolTip(0, QStringLiteral("%1\nClic derecho: aplicar, aplicar y quitar, eliminar").arg(stash.message));
        }
        section->setExpanded(true);
    }
    m_tree->verticalScrollBar()->setValue(scroll);
}

void GitChangesView::updateCommitArea()
{
    const int staged = int(m_status.stagedEntries().size());
    const int changes = int(m_status.changedEntries().size());
    const bool hasText = !m_message->toPlainText().trimmed().isEmpty();
    if (staged > 0) m_commit->setText(QStringLiteral("Guardar (commit) · %1").arg(staged));
    else if (changes > 0) m_commit->setText(QStringLiteral("Guardar todo (commit)"));
    else m_commit->setText(QStringLiteral("Guardar (commit)"));
    m_commit->setEnabled(hasText && (staged > 0 || changes > 0) && m_status.conflicts().isEmpty());
    m_suggest->setEnabled(m_hasAgents && !m_assistantBusy && (staged > 0 || changes > 0) && m_status.conflicts().isEmpty());
    if (!m_hasAgents) m_suggest->setToolTip(QStringLiteral("Conecta un agente (Codex, Claude Code u OpenCode) para recibir sugerencias de mensajes."));
    QString hint;
    if (!m_status.conflicts().isEmpty()) hint = QStringLiteral("Resuelve los conflictos antes de guardar.");
    else if (staged == 0 && changes > 0) hint = QStringLiteral("Nada está preparado: se guardarán todos los cambios. Usa + para elegir archivos.");
    else if (staged > 0) hint = QStringLiteral("Se guardarán solo los archivos de «Listos para guardar». Ctrl+Enter guarda.");
    else hint = QStringLiteral("No hay nada que guardar.");
    const int firstLine = m_message->toPlainText().section(QLatin1Char('\n'), 0, 0).size();
    if (firstLine > 72) hint += QStringLiteral("  El título tiene %1 caracteres: conviene acortarlo (máx. 72).").arg(firstLine);
    m_hint->setText(hint);
    m_commit->setToolTip(staged > 0 ? QStringLiteral("Crea un commit con los archivos listos para guardar: una versión guardada en el historial.")
        : QStringLiteral("Prepara todos los cambios y crea un commit: una versión guardada en el historial."));
}

QList<GitEntry> GitChangesView::selectedEntries() const
{
    QList<GitEntry> entries;
    for (QTreeWidgetItem *item : m_tree->selectedItems()) {
        if (item->parent() == nullptr) continue;
        if (const GitEntry *entry = entryFor(item->data(0, GitChangeRole::Section).toString(), item->data(0, GitChangeRole::Path).toString())) entries << *entry;
    }
    return entries;
}

const GitEntry *GitChangesView::entryFor(const QString &section, const QString &path) const
{
    for (const GitEntry &entry : m_status.entries) {
        if (entry.path != path) continue;
        if (section == QLatin1String("conflicts") && entry.conflicted()) return &entry;
        if (section == QLatin1String("staged") && entry.staged()) return &entry;
        if (section == QLatin1String("changes") && (entry.unstaged() || entry.untracked())) return &entry;
    }
    return nullptr;
}

void GitChangesView::openDiffFor(const QModelIndex &index)
{
    const QString section = index.data(GitChangeRole::Section).toString();
    const QString path = index.data(GitChangeRole::Path).toString();
    if (section == QLatin1String("stashes")) return;
    if (section == QLatin1String("conflicts")) return emit openFileRequested(path);
    GitDiffRequest request;
    request.path = path;
    request.source = section == QLatin1String("staged") ? GitDiffRequest::Source::Staged
        : (index.data(GitChangeRole::Untracked).toBool() ? GitDiffRequest::Source::Untracked : GitDiffRequest::Source::Unstaged);
    emit openDiffRequested(request);
}

void GitChangesView::contextMenu(const QPoint &position)
{
    QTreeWidgetItem *clicked = m_tree->itemAt(position);
    if (clicked == nullptr || clicked->parent() == nullptr) return;
    if (!clicked->isSelected()) {
        m_tree->clearSelection();
        clicked->setSelected(true);
    }
    if (clicked->data(0, GitChangeRole::Section).toString() == QLatin1String("stashes")) {
        const int index = clicked->data(0, GitChangeRole::Path).toInt();
        QMenu stashMenu(this);
        stashMenu.addAction(QStringLiteral("Aplicar (conservar el apartado)"), this, [this, index] { applyStash(index); });
        stashMenu.addAction(QStringLiteral("Aplicar y quitar de la lista (pop)"), this, [this, index] { popStash(index); });
        stashMenu.addSeparator();
        stashMenu.addAction(QStringLiteral("Eliminar…"), this, [this, index] { dropStash(index); });
        stashMenu.exec(m_tree->viewport()->mapToGlobal(position));
        return;
    }
    const QList<GitEntry> entries = selectedEntries();
    if (entries.isEmpty()) return;
    const QString section = clicked->data(0, GitChangeRole::Section).toString();
    const QStringList paths = pathsOf(entries);
    QMenu menu(this);
    menu.addAction(QStringLiteral("Ver los cambios"), this, [this, clicked] { openDiffFor(m_tree->indexFromItem(clicked)); });
    menu.addAction(QStringLiteral("Abrir el archivo"), this, [this, paths] { emit openFileRequested(paths.first()); });
    menu.addSeparator();
    if (section == QLatin1String("staged")) {
        menu.addAction(QStringLiteral("Quitar de los preparados (unstage)"), this, [this, paths] { unstagePaths(paths); });
    } else if (section == QLatin1String("changes")) {
        menu.addAction(QStringLiteral("Preparar para guardar (stage)"), this, [this, paths] { stagePaths(paths); });
        menu.addAction(QStringLiteral("Descartar cambios…"), this, [this, paths] { discardPaths(paths); });
    }
    menu.addSeparator();
    menu.addAction(QStringLiteral("Copiar la ruta"), this, [paths] { QApplication::clipboard()->setText(paths.join(QLatin1Char('\n'))); });
    menu.exec(m_tree->viewport()->mapToGlobal(position));
}

void GitChangesView::stagePaths(const QStringList &paths)
{
    if (paths.isEmpty()) return;
    m_repository->stage(paths, [this](const GitResult &result) { if (!result.ok()) emit message(result.message(), true); });
}

void GitChangesView::unstagePaths(const QStringList &paths)
{
    if (paths.isEmpty()) return;
    m_repository->unstage(paths, m_status.initial, [this](const GitResult &result) { if (!result.ok()) emit message(result.message(), true); });
}

void GitChangesView::discardPaths(const QStringList &paths)
{
    QList<GitEntry> entries;
    for (const GitEntry &entry : m_status.entries) {
        if (paths.contains(entry.path) && (entry.unstaged() || entry.untracked() || entry.staged())) entries << entry;
    }
    if (entries.isEmpty()) return;
    if (m_confirm) {
        int untracked = 0;
        for (const GitEntry &entry : entries) untracked += entry.untracked() ? 1 : 0;
        QString text = QStringLiteral("Se descartarán los cambios de %1 archivo(s) y volverán a su última versión guardada.\n\n"
                                      "Antes guardo una copia de seguridad dentro de .git (carpeta %2) por si te arrepientes.")
                           .arg(entries.size()).arg(gitBackupFolderName());
        if (untracked > 0) text += QStringLiteral("\n\n%1 archivo(s) nuevo(s) sin seguimiento se borrarán del disco.").arg(untracked);
        const auto answer = QMessageBox::warning(this, QStringLiteral("Descartar cambios"), text,
            QMessageBox::Discard | QMessageBox::Cancel, QMessageBox::Cancel);
        if (answer != QMessageBox::Discard) return;
    }
    m_repository->discard(entries, m_status.initial, [this](const GitResult &result, const QString &backup) {
        if (!result.ok()) return emit message(result.message(), true);
        emit message(backup.isEmpty() ? QStringLiteral("Cambios descartados.")
                                      : QStringLiteral("Cambios descartados. Copia de seguridad en %1").arg(backup), false);
    });
}

void GitChangesView::setMessage(const QString &text)
{
    m_message->setPlainText(text);
}

void GitChangesView::commitNow()
{
    const QString text = m_message->toPlainText().trimmed();
    if (text.isEmpty() || !m_commit->isEnabled()) return;
    const bool stageAll = m_status.stagedEntries().isEmpty();
    m_commit->setEnabled(false);
    m_repository->commit(text, stageAll, [this](const GitResult &result) {
        if (!result.ok()) {
            updateCommitArea();
            return emit message(result.message(), true);
        }
        m_message->clear();
        const QRegularExpressionMatch match = QRegularExpression(QStringLiteral(R"(\[([^\]]+)\] (.*))")).match(QString::fromUtf8(result.out));
        emit message(match.hasMatch() ? QStringLiteral("Guardado en %1: %2").arg(match.captured(1), match.captured(2))
                                      : QStringLiteral("Cambios guardados."), false);
    });
}

int GitChangesView::sectionCount(const QString &section) const
{
    for (int i = 0; i < m_tree->topLevelItemCount(); ++i) {
        QTreeWidgetItem *item = m_tree->topLevelItem(i);
        if (item->data(0, GitChangeRole::Section).toString() == section) return item->childCount();
    }
    return 0;
}

QString GitChangesView::commitButtonText() const { return m_commit->text(); }
bool GitChangesView::commitEnabled() const { return m_commit->isEnabled(); }
bool GitChangesView::emptyStateVisible() const { return m_emptyBox->isVisibleTo(this); }

void GitChangesView::setStashes(const QList<GitStash> &stashes)
{
    bool same = stashes.size() == m_stashes.size();
    for (int i = 0; same && i < stashes.size(); ++i) same = stashes.at(i).message == m_stashes.at(i).message && stashes.at(i).time == m_stashes.at(i).time;
    if (same) return;
    m_stashes = stashes;
    rebuild();
}

void GitChangesView::setMergeInProgress(bool inProgress)
{
    m_mergeInProgress = inProgress;
    m_abortMerge->setVisible(inProgress && !m_status.conflicts().isEmpty());
}

bool GitChangesView::abortMergeVisible() const { return m_abortMerge->isVisibleTo(this); }

void GitChangesView::abortMerge()
{
    if (m_confirm && QMessageBox::question(this, QStringLiteral("Cancelar la fusión"),
            QStringLiteral("Se descartará la fusión a medias y el proyecto volverá a como estaba antes de recibir los cambios."),
            QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel) != QMessageBox::Yes) return;
    m_repository->abortMerge([this](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Fusión cancelada.") : result.message(), !result.ok());
    });
}

void GitChangesView::askStash()
{
    if (m_status.changedEntries().isEmpty() && m_status.stagedEntries().isEmpty()) {
        return emit message(QStringLiteral("No hay cambios que apartar."), true);
    }
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Apartar cambios (stash)"));
    auto *layout = new QVBoxLayout(&dialog);
    layout->addWidget(new QLabel(QStringLiteral("Guarda tus cambios aparte y deja el proyecto limpio. Podrás recuperarlos desde «Apartados»."), &dialog));
    auto *name = new QLineEdit(&dialog);
    name->setPlaceholderText(QStringLiteral("Nombre opcional. Ejemplo: arreglo a medias"));
    auto *untracked = new QCheckBox(QStringLiteral("Incluir archivos nuevos (sin seguimiento)"), &dialog);
    untracked->setChecked(true);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(name);
    layout->addWidget(untracked);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    if (dialog.exec() == QDialog::Accepted) stashChanges(name->text(), untracked->isChecked());
}

void GitChangesView::stashChanges(const QString &text, bool includeUntracked)
{
    m_repository->stashPush(text, includeUntracked, [this](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Cambios apartados. Los encuentras en «Apartados (stash)».") : result.message(), !result.ok());
    });
}

void GitChangesView::applyStash(int index)
{
    m_repository->stashApply(index, [this](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Apartado aplicado. Sigue en la lista por si lo necesitas otra vez.") : result.message(), !result.ok());
    });
}

void GitChangesView::popStash(int index)
{
    m_repository->stashPop(index, [this](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Apartado aplicado y quitado de la lista.") : result.message(), !result.ok());
    });
}

void GitChangesView::dropStash(int index)
{
    if (m_confirm && QMessageBox::warning(this, QStringLiteral("Eliminar apartado"),
            QStringLiteral("¿Eliminar este apartado? Los cambios que contiene dejarán de aparecer en la lista."),
            QMessageBox::Discard | QMessageBox::Cancel, QMessageBox::Cancel) != QMessageBox::Discard) return;
    m_repository->stashDrop(index, [this](const GitResult &result) {
        if (!result.ok()) return emit message(result.message(), true);
        const QRegularExpressionMatch match = QRegularExpression(QStringLiteral(R"(\(([0-9a-f]{7,40})\))")).match(QString::fromUtf8(result.out));
        emit message(match.hasMatch() ? QStringLiteral("Apartado eliminado. Si fue un error, recupéralo con: git stash apply %1").arg(match.captured(1))
                                      : QStringLiteral("Apartado eliminado."), false);
    });
}

void GitChangesView::setGuidance(const GitGuidance &guidance)
{
    m_bar->setGuidance(guidance);
}

void GitChangesView::setAssistantAgents(const QList<GitAssistantAgent> &agents, const QString &currentId)
{
    m_hasAgents = !agents.isEmpty();
    m_bar->setAgents(agents, currentId);
    updateCommitArea();
}

void GitChangesView::setAssistantBusy(bool busy, const QString &text)
{
    m_assistantBusy = busy;
    m_bar->setBusy(busy, text);
    updateCommitArea();
}

void GitChangesView::setAssistantNote(const QString &note)
{
    m_note->setText(note);
    m_note->setVisible(!note.isEmpty());
}

bool GitChangesView::suggestEnabled() const { return m_suggest->isEnabled(); }
QString GitChangesView::messageText() const { return m_message->toPlainText(); }
QString GitChangesView::assistantNote() const { return m_note->isVisibleTo(this) ? m_note->text() : QString(); }

void GitChangesView::setAssistantModels(const GitModelList &list, const QString &preference)
{
    m_bar->setModels(list, preference);
}
