#pragma once

#include <QList>
#include <QWidget>

class QGridLayout;

// Lays out its visible widgets in equal columns that adapt to the available
// width (1 to `maxColumns`). Widgets added as full-row span every column.
// Showing or hiding a child reflows the grid, so nothing jumps sideways.
class ResponsiveGrid final : public QWidget {
    Q_OBJECT
public:
    explicit ResponsiveGrid(int minColumnWidth, int maxColumns = 4, QWidget *parent = nullptr);

    void addWidget(QWidget *widget, bool fullRow = false);
    bool hasVisibleItems() const;
    int columns() const { return m_columns; }

protected:
    void resizeEvent(QResizeEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    struct Item {
        QWidget *widget = nullptr;
        bool fullRow = false;
    };
    void relayout(bool force);

    QGridLayout *m_grid = nullptr;
    QList<Item> m_items;
    int m_minColumnWidth = 180;
    int m_maxColumns = 4;
    int m_columns = 0;
    bool m_pending = false;
};
