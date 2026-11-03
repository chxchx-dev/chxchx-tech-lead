#pragma once

#include <QStyledItemDelegate>

#include <functional>

// Item data of a changed file in the Changes tree.
namespace GitChangeRole {
inline constexpr int Path = Qt::UserRole;         // path relative to the repository root
inline constexpr int Section = Qt::UserRole + 1;  // "conflicts" | "staged" | "changes"
inline constexpr int Letter = Qt::UserRole + 2;   // M A D R U ?
inline constexpr int Untracked = Qt::UserRole + 3;
inline constexpr int Title = Qt::UserRole + 4;   // shown instead of the file name (stashes)
inline constexpr int Detail = Qt::UserRole + 5;  // shown instead of the folder
}

// Draws a changed file: status badge, name, folder, and the actions that appear on hover.
class GitChangeDelegate final : public QStyledItemDelegate {
public:
    enum class Action { None, Stage, Unstage, Discard, Apply, Drop };
    using Handler = std::function<void(const QModelIndex &, Action)>;

    explicit GitChangeDelegate(Handler handler, QObject *parent = nullptr);
    void paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const override;
    QSize sizeHint(const QStyleOptionViewItem &option, const QModelIndex &index) const override;
    bool editorEvent(QEvent *event, QAbstractItemModel *model, const QStyleOptionViewItem &option, const QModelIndex &index) override;

    // Actions offered for a section, left to right.
    static QList<Action> actionsFor(const QString &section);
    static QRect actionRect(const QRect &itemRect, int slot);
    static QString tooltipFor(Action action);

private:
    Handler m_handler;
};
