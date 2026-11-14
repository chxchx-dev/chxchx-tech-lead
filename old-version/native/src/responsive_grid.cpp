#include "responsive_grid.hpp"

#include <QEvent>
#include <QGridLayout>
#include <QResizeEvent>
#include <QTimer>

ResponsiveGrid::ResponsiveGrid(int minColumnWidth, int maxColumns, QWidget *parent)
    : QWidget(parent)
    , m_minColumnWidth(minColumnWidth)
    , m_maxColumns(qMax(1, maxColumns))
{
    m_grid = new QGridLayout(this);
    m_grid->setContentsMargins(0, 0, 0, 0);
    m_grid->setHorizontalSpacing(8);
    m_grid->setVerticalSpacing(8);
}

void ResponsiveGrid::addWidget(QWidget *widget, bool fullRow)
{
    widget->setParent(this);
    widget->setSizePolicy(QSizePolicy::Ignored, widget->sizePolicy().verticalPolicy());
    widget->installEventFilter(this);
    m_items.append({widget, fullRow});
    relayout(true);
}

bool ResponsiveGrid::hasVisibleItems() const
{
    for (const Item &item : m_items) {
        if (item.widget->isVisibleTo(const_cast<ResponsiveGrid *>(this))) return true;
    }
    return false;
}

void ResponsiveGrid::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    relayout(false);
}

bool ResponsiveGrid::eventFilter(QObject *watched, QEvent *event)
{
    if ((event->type() == QEvent::ShowToParent || event->type() == QEvent::HideToParent) && !m_pending) {
        // Coalesce the burst of show/hide calls made when an area changes.
        m_pending = true;
        QTimer::singleShot(0, this, [this] {
            m_pending = false;
            relayout(true);
        });
    }
    return QWidget::eventFilter(watched, event);
}

void ResponsiveGrid::relayout(bool force)
{
    const int available = qMax(1, width());
    const int columns = qBound(1, available / m_minColumnWidth, m_maxColumns);
    if (!force && columns == m_columns) return;
    m_columns = columns;
    while (m_grid->count() > 0) delete m_grid->takeAt(0);
    for (int column = 0; column < m_maxColumns; ++column) m_grid->setColumnStretch(column, column < columns ? 1 : 0);
    int row = 0;
    int column = 0;
    for (const Item &item : m_items) {
        if (!item.widget->isVisibleTo(this)) continue;
        if (item.fullRow) {
            if (column > 0) {
                ++row;
                column = 0;
            }
            m_grid->addWidget(item.widget, row++, 0, 1, columns);
            continue;
        }
        m_grid->addWidget(item.widget, row, column);
        if (++column == columns) {
            column = 0;
            ++row;
        }
    }
}
