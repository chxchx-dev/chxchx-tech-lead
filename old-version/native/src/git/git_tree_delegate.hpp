#pragma once

#include "git_explorer.hpp"

#include <QStyledItemDelegate>

// Paints the file explorer with Git state: changed files take the colour of their status and
// show its letter at the right edge; folders with changes inside show a dot.
class GitTreeDelegate final : public QStyledItemDelegate {
public:
    explicit GitTreeDelegate(QObject *parent = nullptr) : QStyledItemDelegate(parent) {}
    void setStatus(GitExplorerStatus status) { m_status = std::move(status); }
    const GitExplorerStatus &status() const { return m_status; }
    void paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const override;
    static QColor colorFor(QChar letter);

protected:
    void initStyleOption(QStyleOptionViewItem *option, const QModelIndex &index) const override;

private:
    GitExplorerStatus m_status;
};
