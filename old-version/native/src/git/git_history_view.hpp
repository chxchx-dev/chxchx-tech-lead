#pragma once

#include "git_diff_view.hpp"
#include "git_types.hpp"

#include <QWidget>

class GitRepository;
class QLabel;
class QListWidget;
class QPushButton;
class QTreeWidget;
class GitGraphDelegate;

// Commit history with the branch graph, the selected commit's message and the files it changed.
class GitHistoryView final : public QWidget {
    Q_OBJECT
public:
    explicit GitHistoryView(GitRepository *repository, QWidget *parent = nullptr);
    void reload();
    // For tests.
    int commitCount() const { return m_commits.size(); }
    const QList<GitCommit> &commits() const { return m_commits; }
    void selectCommit(int row);
    QString detailText() const;
    int fileCount() const;
    bool hasMore() const { return m_more; }
    void loadMore();

signals:
    void openDiffRequested(const GitDiffRequest &request);
    void createBranchRequested(const QString &hash);
    void message(const QString &text, bool error);

private:
    void load(bool append);
    void showDetails(int row);
    void rebuildList(int from);

    GitRepository *m_repository;
    QList<GitCommit> m_commits;
    QVector<QString> m_lanes;
    bool m_more = false;
    int m_generation = 0;
    QListWidget *m_list = nullptr;
    GitGraphDelegate *m_delegate = nullptr;
    QPushButton *m_loadMore = nullptr;
    QLabel *m_detail = nullptr;
    QTreeWidget *m_files = nullptr;
};
