#pragma once

#include "git_types.hpp"

#include <QStyledItemDelegate>

// "hace 5 min", "hace 3 h", "hace 2 d" or the date for older commits.
QString gitRelativeTime(const QDateTime &time, const QDateTime &now = QDateTime::currentDateTime());

// Paints one history row: the branch graph on the left, then ref badges, the subject and
// "hash · author · when". The item's Qt::UserRole holds its index in the commit list.
class GitGraphDelegate final : public QStyledItemDelegate {
public:
    static constexpr int RowHeight = 46;
    explicit GitGraphDelegate(const QList<GitCommit> *commits, QObject *parent = nullptr);
    void setLaneCount(int lanes) { m_lanes = lanes; }
    void paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const override;
    QSize sizeHint(const QStyleOptionViewItem &option, const QModelIndex &index) const override;

private:
    const QList<GitCommit> *m_commits;
    int m_lanes = 1;
};
