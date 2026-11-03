#include "git_branches_view.hpp"

#include "git_repository.hpp"
#include "../studio_icons.hpp"

#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QMessageBox>
#include <QPushButton>
#include <QStyle>
#include <QTreeWidget>
#include <QVBoxLayout>

GitBranchesView::GitBranchesView(GitRepository *repository, QWidget *parent)
    : QWidget(parent), m_repository(repository)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    m_tree = new QTreeWidget(this);
    m_tree->setFrameShape(QFrame::NoFrame);
    m_tree->setHeaderLabels({QStringLiteral("Rama"), QStringLiteral("Último cambio")});
    m_tree->setRootIsDecorated(true);
    m_tree->header()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    layout->addWidget(m_tree, 1);
    auto *buttons = new QHBoxLayout;
    m_new = new QPushButton(studioIcon(QStringLiteral("list-add"), QStyle::SP_FileIcon), QStringLiteral("Nueva…"), this);
    m_new->setToolTip(QStringLiteral("Crea una rama nueva desde donde estás ahora y cambia a ella."));
    m_switch = new QPushButton(studioIcon(QStringLiteral("check"), QStyle::SP_DialogApplyButton), QStringLiteral("Cambiar"), this);
    m_switch->setToolTip(QStringLiteral("Cambia a la rama elegida (checkout). Tus archivos pasan a su versión."));
    m_delete = new QPushButton(studioIcon(QStringLiteral("close-small"), QStyle::SP_DialogCancelButton), QStringLiteral("Borrar"), this);
    m_delete->setToolTip(QStringLiteral("Borra la rama elegida. Solo si sus cambios ya están en otra rama."));
    buttons->addWidget(m_new);
    buttons->addWidget(m_switch);
    buttons->addWidget(m_delete);
    layout->addLayout(buttons);
    connect(m_new, &QPushButton::clicked, this, [this] { askCreate(); });
    connect(m_switch, &QPushButton::clicked, this, [this] { if (const GitBranch *branch = selected()) switchTo(branch->name, branch->remote); });
    connect(m_delete, &QPushButton::clicked, this, [this] { askDelete(); });
    connect(m_tree, &QTreeWidget::itemSelectionChanged, this, [this] { updateButtons(); });
    connect(m_tree, &QTreeWidget::itemDoubleClicked, this, [this](QTreeWidgetItem *item) {
        if (const GitBranch *branch = selected(); branch != nullptr && item->parent() != nullptr) switchTo(branch->name, branch->remote);
    });
    updateButtons();
}

void GitBranchesView::setBranches(const QList<GitBranch> &branches)
{
    const QString previous = selected() != nullptr ? selected()->name : QString();
    m_branches = branches;
    m_tree->clear();
    m_current.clear();
    for (bool remote : {false, true}) {
        auto *group = new QTreeWidgetItem(m_tree, {remote ? QStringLiteral("Remotas") : QStringLiteral("Locales")});
        group->setFlags(Qt::ItemIsEnabled);
        for (const GitBranch &branch : branches) {
            if (branch.remote != remote) continue;
            if (branch.current) m_current = branch.name;
            auto *item = new QTreeWidgetItem(group, {(branch.current ? QStringLiteral("✓ ") : QString()) + branch.name
                    + (branch.track.isEmpty() ? QString() : QStringLiteral("  (%1)").arg(branch.track)), branch.subject});
            item->setData(0, Qt::UserRole, branch.name);
            item->setData(0, Qt::UserRole + 1, branch.remote);
            if (branch.current) {
                QFont bold = item->font(0);
                bold.setBold(true);
                item->setFont(0, bold);
            }
            if (branch.name == previous) item->setSelected(true);
        }
        group->setExpanded(true);
        if (group->childCount() == 0) delete group;
    }
    updateButtons();
}

const GitBranch *GitBranchesView::selected() const
{
    const auto items = m_tree->selectedItems();
    if (items.isEmpty() || items.first()->parent() == nullptr) return nullptr;
    const QString name = items.first()->data(0, Qt::UserRole).toString();
    const bool remote = items.first()->data(0, Qt::UserRole + 1).toBool();
    for (const GitBranch &branch : m_branches) if (branch.name == name && branch.remote == remote) return &branch;
    return nullptr;
}

void GitBranchesView::updateButtons()
{
    const GitBranch *branch = selected();
    m_switch->setEnabled(branch != nullptr && !branch->current);
    m_delete->setEnabled(branch != nullptr && !branch->current && !branch->remote);
}

int GitBranchesView::branchCount(bool remote) const
{
    int count = 0;
    for (const GitBranch &branch : m_branches) count += branch.remote == remote ? 1 : 0;
    return count;
}

void GitBranchesView::switchTo(const QString &name, bool remote)
{
    m_repository->switchBranch(name, remote, [this, name](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Ahora estás en la rama %1.").arg(name) : result.message(), !result.ok());
    });
}

void GitBranchesView::createNamed(const QString &name, const QString &startPoint)
{
    if (name.trimmed().isEmpty()) return;
    m_repository->createBranch(name.trimmed(), [this, name](const GitResult &result) {
        emit message(result.ok() ? QStringLiteral("Rama %1 creada y activa.").arg(name.trimmed()) : result.message(), !result.ok());
    }, startPoint);
}

void GitBranchesView::askCreate()
{
    bool accepted = false;
    const QString name = QInputDialog::getText(this, QStringLiteral("Nueva rama"),
        QStringLiteral("Nombre de la rama (sin espacios; por ejemplo arreglo-login).\nSe crea desde donde estás ahora y cambias a ella."),
        QLineEdit::Normal, QString(), &accepted);
    if (accepted) createNamed(name);
}

void GitBranchesView::askDelete()
{
    const GitBranch *branch = selected();
    if (branch == nullptr || branch->current || branch->remote) return;
    if (m_confirm && QMessageBox::question(this, QStringLiteral("Borrar rama"),
            QStringLiteral("¿Borrar la rama %1? Solo se borra si sus cambios ya están en otra rama.").arg(branch->name),
            QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel) != QMessageBox::Yes) return;
    deleteNamed(branch->name, false);
}

void GitBranchesView::deleteNamed(const QString &name, bool force)
{
    m_repository->deleteBranch(name, force, [this, name, force](const GitResult &result) {
        if (result.ok()) return emit message(QStringLiteral("Rama %1 borrada.").arg(name), false);
        const bool unmerged = result.err.contains(QLatin1String("not fully merged"));
        if (unmerged && !force && m_confirm
            && QMessageBox::warning(this, QStringLiteral("La rama tiene commits sin integrar"),
                QStringLiteral("La rama %1 tiene commits que no están en ninguna otra rama. Si la borras, esos commits se perderán.\n\n¿Borrarla de todos modos?").arg(name),
                QMessageBox::Discard | QMessageBox::Cancel, QMessageBox::Cancel) == QMessageBox::Discard) {
            return deleteNamed(name, true);
        }
        emit message(result.message(), true);
    });
}
