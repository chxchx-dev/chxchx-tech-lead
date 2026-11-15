#include "git_graph_delegate.hpp"

#include "../studio_theme.hpp"

#include <QPainter>
#include <QPainterPath>

namespace {

constexpr int LaneWidth = 14;
constexpr int GraphPadding = 10;
constexpr int MaxDrawnLanes = 8;

QColor laneColor(int lane)
{
    static const QColor palette[] = {QColor("#58a6ff"), QColor("#3fb950"), QColor("#d29922"), QColor("#f85149"),
                                     QColor("#bc8cff"), QColor("#39c5cf"), QColor("#f778ba"), QColor("#a5d6a7")};
    return palette[lane % 8];
}

int laneX(const QRect &rect, int lane)
{
    return rect.left() + GraphPadding + qMin(lane, MaxDrawnLanes - 1) * LaneWidth;
}

void drawEdge(QPainter &painter, QPointF from, QPointF to, const QColor &color)
{
    painter.setPen(QPen(color, 2));
    if (qFuzzyCompare(from.x(), to.x())) {
        painter.drawLine(from, to);
        return;
    }
    QPainterPath path(from);
    const qreal middle = (from.y() + to.y()) / 2;
    path.cubicTo(QPointF(from.x(), middle), QPointF(to.x(), middle), to);
    painter.drawPath(path);
}

QColor refColor(const QString &ref)
{
    const StudioTheme &theme = ThemeManager::current();
    if (ref.startsWith(QLatin1String("HEAD"))) return theme.accent;
    if (ref.startsWith(QLatin1String("tag:"))) return theme.warning;
    if (ref.contains(QLatin1Char('/'))) return QColor("#58a6ff");
    return theme.success;
}

} // namespace

QString gitRelativeTime(const QDateTime &time, const QDateTime &now)
{
    const qint64 seconds = time.secsTo(now);
    if (seconds < 60) return QStringLiteral("ahora");
    if (seconds < 3600) return QStringLiteral("hace %1 min").arg(seconds / 60);
    if (seconds < 86400) return QStringLiteral("hace %1 h").arg(seconds / 3600);
    if (seconds < 30 * 86400) return QStringLiteral("hace %1 d").arg(seconds / 86400);
    return time.date().toString(QStringLiteral("yyyy-MM-dd"));
}

GitGraphDelegate::GitGraphDelegate(const QList<GitCommit> *commits, QObject *parent)
    : QStyledItemDelegate(parent), m_commits(commits)
{
}

QSize GitGraphDelegate::sizeHint(const QStyleOptionViewItem &, const QModelIndex &) const
{
    return {0, RowHeight};
}

void GitGraphDelegate::paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const
{
    const int row = index.data(Qt::UserRole).toInt();
    if (row < 0 || row >= m_commits->size()) return;
    const GitCommit &commit = m_commits->at(row);
    const GitGraphRow &graph = commit.graph;
    const StudioTheme &theme = ThemeManager::current();
    painter->save();
    painter->setRenderHint(QPainter::Antialiasing);
    if (option.state & QStyle::State_Selected) painter->fillRect(option.rect, theme.accentSoft);
    else if (option.state & QStyle::State_MouseOver) painter->fillRect(option.rect, theme.surfaceHover);

    const QRect rect = option.rect;
    const qreal top = rect.top();
    const qreal middle = rect.top() + rect.height() / 2.0;
    const qreal bottom = rect.bottom() + 1;
    painter->setClipRect(rect);
    for (int lane : graph.through) {
        drawEdge(*painter, {qreal(laneX(rect, lane)), top}, {qreal(laneX(rect, lane)), bottom}, laneColor(lane));
    }
    const qreal nodeX = laneX(rect, graph.lane);
    if (graph.incoming) drawEdge(*painter, {nodeX, top}, {nodeX, middle}, laneColor(graph.lane));
    for (int lane : graph.mergeIn) drawEdge(*painter, {qreal(laneX(rect, lane)), top}, {nodeX, middle}, laneColor(lane));
    for (int target : graph.parentLanes) {
        drawEdge(*painter, {nodeX, middle}, {qreal(laneX(rect, target)), bottom}, laneColor(target == graph.lane ? graph.lane : target));
    }
    const bool merge = commit.parents.size() > 1;
    const bool head = !commit.refs.isEmpty() && commit.refs.first().startsWith(QLatin1String("HEAD"));
    painter->setPen(QPen(laneColor(graph.lane), head ? 3 : 2));
    painter->setBrush(merge ? theme.background : laneColor(graph.lane));
    painter->drawEllipse(QPointF(nodeX, middle), 4.5, 4.5);

    const int textLeft = rect.left() + GraphPadding * 2 + qMin(m_lanes, MaxDrawnLanes) * LaneWidth;
    QFont small = option.font;
    small.setPointSizeF(small.pointSizeF() * 0.85);
    const QFontMetrics metrics(option.font);
    int x = textLeft;
    const int y1 = rect.top() + 4;
    for (const QString &ref : commit.refs) {
        const QString label = ref.startsWith(QLatin1String("HEAD -> ")) ? ref.mid(8) : ref;
        painter->setFont(small);
        const QFontMetrics badgeMetrics(small);
        const int width = badgeMetrics.horizontalAdvance(label) + 12;
        if (x + width > rect.right() - 6) break;
        const QRect badge(x, y1 + 1, width, metrics.height() + 2);
        painter->setPen(Qt::NoPen);
        QColor fill = refColor(ref);
        fill.setAlpha(60);
        painter->setBrush(fill);
        painter->drawRoundedRect(badge, 4, 4);
        painter->setPen(refColor(ref));
        painter->drawText(badge, Qt::AlignCenter, label);
        x += width + 4;
    }
    painter->setFont(option.font);
    painter->setPen((option.state & QStyle::State_Selected) ? theme.selectedText : theme.text);
    const QRect subject(x, y1, rect.right() - x - 6, metrics.height() + 4);
    painter->drawText(subject, Qt::AlignVCenter | Qt::AlignLeft, metrics.elidedText(commit.subject, Qt::ElideRight, subject.width()));
    painter->setFont(small);
    painter->setPen(theme.textMuted);
    const QString meta = QStringLiteral("%1 · %2 · %3").arg(commit.shortHash(), commit.author, gitRelativeTime(commit.time));
    const QRect metaRect(textLeft, y1 + metrics.height() + 6, rect.right() - textLeft - 6, QFontMetrics(small).height() + 2);
    painter->drawText(metaRect, Qt::AlignVCenter | Qt::AlignLeft, QFontMetrics(small).elidedText(meta, Qt::ElideRight, metaRect.width()));
    painter->restore();
}
