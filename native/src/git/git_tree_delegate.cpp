#include "git_tree_delegate.hpp"

#include "../studio_theme.hpp"

#include <QFileSystemModel>
#include <QPainter>

QColor GitTreeDelegate::colorFor(QChar letter)
{
    const StudioTheme &theme = ThemeManager::current();
    switch (letter.toLatin1()) {
    case 'N': case 'A': return theme.success;
    case 'D': case 'U': return theme.danger;
    case 'R': case 'C': return theme.accent;
    default: return theme.warning;
    }
}

void GitTreeDelegate::initStyleOption(QStyleOptionViewItem *option, const QModelIndex &index) const
{
    QStyledItemDelegate::initStyleOption(option, index);
    const QString path = index.data(QFileSystemModel::FilePathRole).toString();
    const auto file = m_status.files.constFind(path);
    if (file != m_status.files.constEnd()) option->palette.setColor(QPalette::Text, colorFor(file.value()));
}

void GitTreeDelegate::paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const
{
    QStyledItemDelegate::paint(painter, option, index);
    if (m_status.isEmpty()) return;
    const QString path = index.data(QFileSystemModel::FilePathRole).toString();
    const auto file = m_status.files.constFind(path);
    const bool folder = file == m_status.files.constEnd() && m_status.folders.contains(path);
    if (file == m_status.files.constEnd() && !folder) return;
    painter->save();
    painter->setRenderHint(QPainter::Antialiasing);
    const QRect rect = option.rect;
    if (folder) {
        QColor dot = ThemeManager::current().warning;
        dot.setAlpha(190);
        painter->setPen(Qt::NoPen);
        painter->setBrush(dot);
        painter->drawEllipse(QPointF(rect.right() - 12, rect.center().y()), 3.0, 3.0);
    } else {
        QFont font = option.font;
        font.setBold(true);
        font.setPointSizeF(font.pointSizeF() * 0.85);
        painter->setFont(font);
        painter->setPen(colorFor(file.value()));
        painter->drawText(QRect(rect.right() - 22, rect.top(), 16, rect.height()), Qt::AlignCenter, QString(file.value()));
    }
    painter->restore();
}
