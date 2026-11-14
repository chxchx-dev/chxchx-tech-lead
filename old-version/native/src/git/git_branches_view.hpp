#pragma once

#include "git_types.hpp"

#include <QWidget>

class GitRepository;
class QPushButton;
class QTreeWidget;

// The Branches tab: switch, create and delete branches.
class GitBranchesView final : public QWidget {
    Q_OBJECT
public:
    explicit GitBranchesView(GitRepository *repository, QWidget *parent = nullptr);
    void setBranches(const QList<GitBranch> &branches);
    void setConfirmations(bool enabled) { m_confirm = enabled; }
    // Actions without dialogs, used by the buttons and by tests.
    void switchTo(const QString &name, bool remote);
    void createNamed(const QString &name, const QString &startPoint = QString());
    void deleteNamed(const QString &name, bool force = false);
    int branchCount(bool remote) const;
    QString currentBranch() const { return m_current; }

signals:
    void message(const QString &text, bool error);

private:
    const GitBranch *selected() const;
    void askCreate();
    void askDelete();
    void updateButtons();

    GitRepository *m_repository;
    QList<GitBranch> m_branches;
    QString m_current;
    bool m_confirm = true;
    QTreeWidget *m_tree = nullptr;
    QPushButton *m_new = nullptr;
    QPushButton *m_switch = nullptr;
    QPushButton *m_delete = nullptr;
};
