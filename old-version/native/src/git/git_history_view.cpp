#include "git_history_view.hpp"

#include "git_graph_delegate.hpp"
#include "git_parsers.hpp"
#include "git_repository.hpp"
#include "../studio_theme.hpp"

#include <QApplication>
#include <QClipboard>
#include <QLabel>
#include <QListWidget>
#include <QMenu>
#include <QPushButton>
#include <QSplitter>
#include <QTreeWidget>
#include <QVBoxLayout>

namespace {

constexpr int PageSize = 200;

QColor statusColor(const QString &status)
{
    const StudioTheme &theme = ThemeManager::current();
    if (status == QLatin1String("A")) return theme.success;
    if (status == QLatin1String("D")) return theme.danger;
    if (status == QLatin1String("R") || status == QLatin1String("C")) return theme.accent;
    return theme.warning;
}

} // namespace

GitHistoryView::GitHistoryView(GitRepository *repository, QWidget *parent)
    : QWidget(parent), m_repository(repository)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    auto *splitter = new QSplitter(Qt::Vertical, this);
    splitter->setChildrenCollapsible(false);
    layout->addWidget(splitter);

    auto *top = new QWidget(splitter);
    auto *topLayout = new QVBoxLayout(top);
    topLayout->setContentsMargins(0, 0, 0, 0);
    topLayout->setSpacing(0);
    m_list = new QListWidget(top);
    m_list->setFrameShape(QFrame::NoFrame);
    m_list->setMouseTracking(true);
    m_list->setUniformItemSizes(true);
    m_list->setContextMenuPolicy(Qt::CustomContextMenu);
    m_delegate = new GitGraphDelegate(&m_commits, this);
    m_list->setItemDelegate(m_delegate);
    m_loadMore = new QPushButton(QStringLiteral("Cargar commits anteriores"), top);
    m_loadMore->hide();
    topLayout->addWidget(m_list, 1);
    topLayout->addWidget(m_loadMore);

    auto *bottom = new QWidget(splitter);
    auto *bottomLayout = new QVBoxLayout(bottom);
    bottomLayout->setContentsMargins(8, 6, 8, 6);
    m_detail = new QLabel(QStringLiteral("Elige un commit para ver su mensaje y los archivos que cambió."), bottom);
    m_detail->setWordWrap(true);
    m_detail->setTextInteractionFlags(Qt::TextSelectableByMouse);
    m_detail->setAlignment(Qt::AlignTop | Qt::AlignLeft);
    m_files = new QTreeWidget(bottom);
    m_files->setHeaderHidden(true);
    m_files->setRootIsDecorated(false);
    m_files->setFrameShape(QFrame::NoFrame);
    bottomLayout->addWidget(m_detail);
    bottomLayout->addWidget(m_files, 1);
    splitter->setStretchFactor(0, 3);
    splitter->setStretchFactor(1, 2);

    connect(m_list, &QListWidget::currentRowChanged, this, [this](int row) { showDetails(row); });
    connect(m_loadMore, &QPushButton::clicked, this, [this] { loadMore(); });
    connect(m_files, &QTreeWidget::itemClicked, this, [this](QTreeWidgetItem *item) {
        const int row = m_list->currentRow();
        if (row < 0 || row >= m_commits.size()) return;
        GitDiffRequest request;
        request.source = GitDiffRequest::Source::Commit;
        request.hash = m_commits.at(row).hash;
        request.path = item->data(0, Qt::UserRole).toString();
        emit openDiffRequested(request);
    });
    connect(m_list, &QListWidget::customContextMenuRequested, this, [this](const QPoint &position) {
        QListWidgetItem *item = m_list->itemAt(position);
        if (item == nullptr) return;
        const QString hash = m_commits.value(item->data(Qt::UserRole).toInt()).hash;
        QMenu menu(this);
        menu.addAction(QStringLiteral("Copiar el identificador del commit (hash)"), this, [hash] { QApplication::clipboard()->setText(hash); });
        menu.addAction(QStringLiteral("Crear una rama desde este commit…"), this, [this, hash] { emit createBranchRequested(hash); });
        menu.exec(m_list->viewport()->mapToGlobal(position));
    });
}

void GitHistoryView::reload()
{
    load(false);
}

void GitHistoryView::loadMore()
{
    load(true);
}

void GitHistoryView::load(bool append)
{
    const int generation = ++m_generation;
    const int skip = append ? m_commits.size() : 0;
    m_repository->log(skip, PageSize, [this, generation, append, skip](const GitResult &result, const QList<GitCommit> &page) {
        if (generation != m_generation) return; // a newer reload superseded this one
        if (!result.ok()) {
            // A repository without commits has no log yet: that is empty history, not an error.
            if (!append) {
                m_commits.clear();
                m_list->clear();
                m_detail->setText(result.err.contains(QLatin1String("does not have any commits"))
                    ? QStringLiteral("Todavía no hay commits. Guarda tus primeros cambios desde la pestaña Cambios.") : result.message());
            }
            return;
        }
        if (!append) {
            m_commits.clear();
            m_lanes.clear();
        }
        QList<GitCommit> added = page;
        assignGraph(added, m_lanes);
        m_commits += added;
        m_more = page.size() == PageSize;
        m_loadMore->setVisible(m_more);
        rebuildList(skip);
    });
}

void GitHistoryView::rebuildList(int from)
{
    int lanes = 1;
    for (const GitCommit &commit : m_commits) lanes = qMax(lanes, commit.graph.laneCount);
    m_delegate->setLaneCount(lanes);
    const int selected = m_list->currentRow();
    if (from == 0) m_list->clear();
    for (int i = from; i < m_commits.size(); ++i) {
        auto *item = new QListWidgetItem(m_list);
        item->setData(Qt::UserRole, i);
        item->setToolTip(m_commits.at(i).subject);
    }
    if (from == 0 && selected >= 0 && selected < m_list->count()) m_list->setCurrentRow(selected);
    m_list->viewport()->update();
}

void GitHistoryView::selectCommit(int row)
{
    m_list->setCurrentRow(row);
}

void GitHistoryView::showDetails(int row)
{
    m_files->clear();
    if (row < 0 || row >= m_commits.size()) return;
    const GitCommit commit = m_commits.at(row);
    m_detail->setText(QStringLiteral("<b>%1</b><br>%2").arg(commit.subject.toHtmlEscaped(), commit.shortHash()));
    const int generation = m_generation;
    m_repository->commitMessage(commit.hash, [this, generation, commit](const GitResult &, const QString &body) {
        if (generation != m_generation || m_list->currentRow() < 0 || m_commits.value(m_list->currentRow()).hash != commit.hash) return;
        m_detail->setText(QStringLiteral("<pre style='white-space:pre-wrap'>%1</pre><span style='color:gray'>%2 · %3 · %4</span>")
            .arg(body.toHtmlEscaped(), commit.shortHash(), commit.author.toHtmlEscaped(), commit.time.toString(QStringLiteral("yyyy-MM-dd HH:mm"))));
    });
    m_repository->commitFiles(commit.hash, [this, commit](const GitResult &, const QList<GitFileChange> &files) {
        if (m_list->currentRow() < 0 || m_commits.value(m_list->currentRow()).hash != commit.hash) return;
        m_files->clear();
        for (const GitFileChange &change : files) {
            auto *item = new QTreeWidgetItem(m_files);
            item->setText(0, QStringLiteral("%1  %2").arg(change.status, change.path));
            item->setForeground(0, statusColor(change.status));
            item->setData(0, Qt::UserRole, change.path);
            item->setToolTip(0, QStringLiteral("%1 · clic para ver los cambios").arg(changeLabel(change.status.at(0))));
        }
    });
}

QString GitHistoryView::detailText() const
{
    return m_detail->text();
}

int GitHistoryView::fileCount() const
{
    return m_files->topLevelItemCount();
}
