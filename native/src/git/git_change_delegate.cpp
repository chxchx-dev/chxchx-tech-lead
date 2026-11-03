#include "git_change_delegate.hpp"

#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QCursor>
#include <QMouseEvent>
#include <QPainter>
#include <QToolTip>

namespace {

constexpr int RowHeight = 28;
constexpr int ActionSize = 22;

QColor letterColor(const QString &letter, bool untracked)
{
    const StudioTheme &theme = ThemeManager::current();
    if (untracked || letter == QLatin1String("A")) return theme.success;
    if (letter == QLatin1String("D") || letter == QLatin1String("U")) return theme.danger;
    if (letter == QLatin1String("R") || letter == QLatin1String("C") || letter == QLatin1String("S")) return theme.accent;
    return theme.warning;
}

QString iconName(GitChangeDelegate::Action action)
{
    switch (action) {
    case GitChangeDelegate::Action::Stage: return QStringLiteral("list-add");
    case GitChangeDelegate::Action::Unstage: return QStringLiteral("list-remove");
    case GitChangeDelegate::Action::Discard: return QStringLiteral("edit-undo");
    case GitChangeDelegate::Action::Apply: return QStringLiteral("go-down");
    case GitChangeDelegate::Action::Drop: return QStringLiteral("close-small");
    case GitChangeDelegate::Action::None: break;
    }
    return {};
}

} // namespace

GitChangeDelegate::GitChangeDelegate(Handler handler, QObject *parent)
    : QStyledItemDelegate(parent), m_handler(std::move(handler))
{
}

QList<GitChangeDelegate::Action> GitChangeDelegate::actionsFor(const QString &section)
{
    if (section == QLatin1String("staged")) return {Action::Unstage};
    if (section == QLatin1String("changes")) return {Action::Discard, Action::Stage};
    if (section == QLatin1String("stashes")) return {Action::Drop, Action::Apply};
    return {};
}

QRect GitChangeDelegate::actionRect(const QRect &itemRect, int slot)
{
    return QRect(itemRect.right() - 6 - (slot + 1) * (ActionSize + 2), itemRect.top() + (itemRect.height() - ActionSize) / 2, ActionSize, ActionSize);
}

QString GitChangeDelegate::tooltipFor(Action action)
{
    switch (action) {
    case Action::Stage: return QStringLiteral("Preparar para guardar (stage)");
    case Action::Unstage: return QStringLiteral("Quitar de los preparados (unstage)");
    case Action::Discard: return QStringLiteral("Descartar cambios (se guarda una copia de seguridad)");
    case Action::Apply: return QStringLiteral("Aplicar este apartado y conservarlo en la lista");
    case Action::Drop: return QStringLiteral("Eliminar este apartado");
    case Action::None: break;
    }
    return {};
}

QSize GitChangeDelegate::sizeHint(const QStyleOptionViewItem &, const QModelIndex &) const
{
    return {0, RowHeight};
}

void GitChangeDelegate::paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const
{
    const StudioTheme &theme = ThemeManager::current();
    const QString path = index.data(GitChangeRole::Path).toString();
    const QString section = index.data(GitChangeRole::Section).toString();
    const QString letter = index.data(GitChangeRole::Letter).toString();
    const bool untracked = index.data(GitChangeRole::Untracked).toBool();
    const bool selected = option.state & QStyle::State_Selected;
    const bool hovered = option.state & QStyle::State_MouseOver;
    painter->save();
    painter->setRenderHint(QPainter::Antialiasing);
    if (selected) painter->fillRect(option.rect, theme.accentSoft);
    else if (hovered) painter->fillRect(option.rect, theme.surfaceHover);

    const QRect rect = option.rect;
    const QRect badge(rect.left() + 10, rect.top() + (rect.height() - 18) / 2, 18, 18);
    QColor fill = letterColor(letter, untracked);
    fill.setAlpha(55);
    painter->setPen(Qt::NoPen);
    painter->setBrush(fill);
    painter->drawRoundedRect(badge, 4, 4);
    painter->setPen(letterColor(letter, untracked));
    QFont badgeFont = option.font;
    badgeFont.setBold(true);
    badgeFont.setPointSizeF(badgeFont.pointSizeF() * 0.85);
    painter->setFont(badgeFont);
    painter->drawText(badge, Qt::AlignCenter, untracked ? QStringLiteral("N") : letter);

    const QList<Action> actions = actionsFor(section);
    const int reserved = (hovered || selected) ? int(actions.size()) * (ActionSize + 2) + 8 : 6;
    const QString title = index.data(GitChangeRole::Title).toString();
    const QString name = title.isEmpty() ? path.section(QLatin1Char('/'), -1) : title;
    const QString folder = title.isEmpty() ? (path.size() > name.size() ? path.left(path.size() - name.size() - 1) : QString())
                                           : index.data(GitChangeRole::Detail).toString();
    const QFontMetrics metrics(option.font);
    int x = badge.right() + 8;
    QFont small = option.font;
    small.setPointSizeF(small.pointSizeF() * 0.88);
    // A title row (a stash) keeps its age fully visible at the right; the title gives way instead.
    const int detailWidth = title.isEmpty() || folder.isEmpty() ? 0 : QFontMetrics(small).horizontalAdvance(folder) + 10;
    const int available = rect.right() - reserved - x - detailWidth;
    painter->setFont(option.font);
    painter->setPen(selected ? theme.selectedText : theme.text);
    const QString shownName = metrics.elidedText(name, Qt::ElideMiddle, available);
    painter->drawText(QRect(x, rect.top(), available, rect.height()), Qt::AlignVCenter | Qt::AlignLeft, shownName);
    if (detailWidth > 0) {
        painter->setFont(small);
        painter->setPen(theme.textMuted);
        painter->drawText(QRect(rect.right() - reserved - detailWidth + 4, rect.top(), detailWidth, rect.height()), Qt::AlignVCenter | Qt::AlignRight, folder);
        painter->restore();
        return;
    }
    x += metrics.horizontalAdvance(shownName) + 8;
    if (!folder.isEmpty() && x < rect.right() - reserved - 20) {
        painter->setFont(small);
        painter->setPen(theme.textMuted);
        const int room = rect.right() - reserved - x;
        painter->drawText(QRect(x, rect.top(), room, rect.height()), Qt::AlignVCenter | Qt::AlignLeft,
                          QFontMetrics(small).elidedText(folder, Qt::ElideLeft, room));
    }
    if (hovered || selected) {
        const QPoint cursor = option.widget != nullptr ? option.widget->mapFromGlobal(QCursor::pos()) : QPoint(-1, -1);
        painter->setFont(option.font);
        for (int slot = 0; slot < actions.size(); ++slot) {
            const QRect button = actionRect(rect, slot);
            const bool over = button.contains(cursor);
            painter->setPen(Qt::NoPen);
            painter->setBrush(over ? theme.surfaceRaised : Qt::transparent);
            painter->drawRoundedRect(button, 4, 4);
            const QPixmap icon = renderStudioIcon(iconName(actions.at(slot)), over ? theme.accent : theme.textMuted, 16);
            painter->drawPixmap(button.center() - QPoint(8, 8), icon);
        }
    }
    painter->restore();
}

bool GitChangeDelegate::editorEvent(QEvent *event, QAbstractItemModel *model, const QStyleOptionViewItem &option, const QModelIndex &index)
{
    if (event->type() == QEvent::MouseButtonRelease || event->type() == QEvent::ToolTip) {
        const QList<Action> actions = actionsFor(index.data(GitChangeRole::Section).toString());
        const QPoint position = event->type() == QEvent::MouseButtonRelease
            ? static_cast<QMouseEvent *>(event)->position().toPoint() : QPoint(-1, -1);
        for (int slot = 0; slot < actions.size(); ++slot) {
            if (event->type() == QEvent::MouseButtonRelease && actionRect(option.rect, slot).contains(position)) {
                if (m_handler) m_handler(index, actions.at(slot));
                return true; // consumed: do not also open the diff
            }
        }
    }
    return QStyledItemDelegate::editorEvent(event, model, option, index);
}
