#include "studio_icons.hpp"
#include "studio_theme.hpp"

#include <QApplication>
#include <QHash>
#include <QIconEngine>
#include <QPainter>
#include <QPainterPath>

#include <functional>

namespace {

// Drawing helpers on a 24×24 grid; the painter is already scaled.
struct Pen {
    QPainter &p;
    void line(qreal x1, qreal y1, qreal x2, qreal y2) { p.drawLine(QPointF(x1, y1), QPointF(x2, y2)); }
    void poly(std::initializer_list<QPointF> points) { p.drawPolyline(points.begin(), int(points.size())); }
    void closed(std::initializer_list<QPointF> points) { p.drawPolygon(points.begin(), int(points.size())); }
    void rect(qreal x, qreal y, qreal w, qreal h, qreal r = 2) { p.drawRoundedRect(QRectF(x, y, w, h), r, r); }
    void circle(qreal cx, qreal cy, qreal r) { p.drawEllipse(QPointF(cx, cy), r, r); }
    void dot(qreal cx, qreal cy, qreal r = 1.1)
    {
        p.save();
        p.setBrush(p.pen().color());
        p.setPen(Qt::NoPen);
        p.drawEllipse(QPointF(cx, cy), r, r);
        p.restore();
    }
    void arc(qreal cx, qreal cy, qreal r, int startDeg, int spanDeg)
    {
        p.drawArc(QRectF(cx - r, cy - r, 2 * r, 2 * r), startDeg * 16, spanDeg * 16);
    }
};

using Draw = std::function<void(Pen &)>;

void page(Pen &d)
{
    d.poly({{14, 3}, {6, 3}, {5, 4}, {5, 20}, {6, 21}, {18, 21}, {19, 20}, {19, 8}, {14, 3}});
    d.poly({{14, 3}, {14, 8}, {19, 8}});
}

void folder(Pen &d)
{
    d.poly({{3, 7}, {3, 18}, {4, 19}, {20, 19}, {21, 18}, {21, 9}, {20, 8}, {12, 8}, {10, 5}, {4, 5}, {3, 6}, {3, 7}});
}

void bubble(Pen &d)
{
    d.poly({{4, 5}, {20, 5}, {21, 6}, {21, 15}, {20, 16}, {11, 16}, {7, 20}, {7, 16}, {4, 16}, {3, 15}, {3, 6}, {4, 5}});
}

const QHash<QString, Draw> &drawings()
{
    static const QHash<QString, Draw> icons = {
        {QStringLiteral("folder"), folder},
        {QStringLiteral("folder-open"), [](Pen &d) {
            d.poly({{3, 18}, {3, 6}, {4, 5}, {10, 5}, {12, 8}, {18, 8}, {19, 9}, {19, 11}});
            d.poly({{3, 18}, {6, 11}, {21, 11}, {18, 18}, {3, 18}});
        }},
        {QStringLiteral("folders"), [](Pen &d) {
            d.poly({{6, 8}, {6, 17}, {7, 18}, {20, 18}, {21, 17}, {21, 10}, {20, 9}, {13, 9}, {11.5, 6.5}, {7, 6.5}, {6, 7.5}});
            d.poly({{3, 10}, {3, 20}, {4, 21}, {17, 21}});
        }},
        {QStringLiteral("document-open"), folder},
        {QStringLiteral("document-new"), [](Pen &d) { page(d); d.line(12, 11, 12, 17); d.line(9, 14, 15, 14); }},
        {QStringLiteral("text-x-generic"), [](Pen &d) { page(d); d.line(9, 13, 15, 13); d.line(9, 17, 13, 17); }},
        {QStringLiteral("text-x-script"), [](Pen &d) {
            page(d);
            d.poly({{10, 12}, {8, 14.5}, {10, 17}});
            d.poly({{14, 12}, {16, 14.5}, {14, 17}});
        }},
        {QStringLiteral("document-save"), [](Pen &d) {
            d.poly({{5, 4}, {16, 4}, {20, 8}, {20, 19}, {19, 20}, {5, 20}, {4, 19}, {4, 5}, {5, 4}});
            d.poly({{8, 4}, {8, 8}, {15, 8}, {15, 4}});
            d.rect(8, 13, 8, 7, 1);
        }},
        {QStringLiteral("document-save-as"), [](Pen &d) {
            d.poly({{11, 20}, {5, 20}, {4, 19}, {4, 5}, {5, 4}, {16, 4}, {20, 8}, {20, 11}});
            d.poly({{8, 4}, {8, 8}, {15, 8}, {15, 4}});
            d.poly({{14, 20}, {14, 17.5}, {19, 12.5}, {21.5, 15}, {16.5, 20}, {14, 20}});
        }},
        {QStringLiteral("system-search"), [](Pen &d) { d.circle(10.5, 10.5, 6); d.line(15, 15, 20, 20); }},
        {QStringLiteral("edit-find"), [](Pen &d) {
            d.poly({{11, 21}, {6, 21}, {5, 20}, {5, 4}, {6, 3}, {14, 3}, {19, 8}, {19, 11}});
            d.circle(15, 15.5, 3.5);
            d.line(17.6, 18.1, 20.5, 21);
        }},
        {QStringLiteral("window-float"), [](Pen &d) { d.rect(8, 4, 12, 12, 2); d.poly({{8, 8}, {5, 8}, {4, 9}, {4, 19}, {5, 20}, {15, 20}, {16, 19}, {16, 16}}); }},
        {QStringLiteral("view-refresh"), [](Pen &d) {
            d.arc(12, 12, 7.5, 40, 280);
            d.poly({{19.5, 4.5}, {19.5, 9.5}, {14.5, 9.5}});
        }},
        {QStringLiteral("utilities-terminal"), [](Pen &d) {
            d.rect(3, 5, 18, 14, 2.5);
            d.poly({{7, 10}, {10, 12.5}, {7, 15}});
            d.line(12, 15, 16, 15);
        }},
        {QStringLiteral("media-playback-start"), [](Pen &d) { d.closed({{8, 5}, {19, 12}, {8, 19}}); }},
        {QStringLiteral("media-playback-pause"), [](Pen &d) { d.rect(7, 5, 3, 14, 1); d.rect(14, 5, 3, 14, 1); }},
        {QStringLiteral("media-playback-stop"), [](Pen &d) { d.rect(6, 6, 12, 12, 2); }},
        {QStringLiteral("view-list-details"), [](Pen &d) { d.rect(3, 4, 18, 16, 2.5); d.line(9, 4, 9, 20); }},
        {QStringLiteral("view-dual"), [](Pen &d) { d.rect(3, 4, 18, 16, 2.5); d.line(15, 4, 15, 20); }},
        {QStringLiteral("view-history"), [](Pen &d) { d.rect(3, 4, 18, 16, 2.5); d.line(3, 14, 21, 14); }},
        {QStringLiteral("dialog-information"), [](Pen &d) { d.circle(12, 12, 9); d.line(12, 11, 12, 16.5); d.dot(12, 7.8); }},
        {QStringLiteral("help-contents"), [](Pen &d) {
            d.circle(12, 12, 9);
            d.arc(12, 9.5, 2.6, -60, 240);
            d.poly({{13.3, 11.7}, {12, 13}, {12, 14}});
            d.dot(12, 16.8);
        }},
        {QStringLiteral("dialog-warning"), [](Pen &d) {
            d.closed({{12, 3.5}, {21.5, 20}, {2.5, 20}});
            d.line(12, 10, 12, 14.5);
            d.dot(12, 17.3);
        }},
        {QStringLiteral("security-high"), [](Pen &d) {
            d.poly({{12, 3}, {19, 6}, {19, 11}, {18, 15}, {15, 18.5}, {12, 21}, {9, 18.5}, {6, 15}, {5, 11}, {5, 6}, {12, 3}});
            d.poly({{9, 12}, {11, 14}, {15, 10}});
        }},
        {QStringLiteral("dialog-password"), [](Pen &d) {
            d.rect(5, 10, 14, 10, 2);
            d.poly({{8, 10}, {8, 7}, {9.5, 4.5}, {12, 3.5}, {14.5, 4.5}, {16, 7}, {16, 10}});
        }},
        {QStringLiteral("dialog-messages"), bubble},
        {QStringLiteral("chat-message"), bubble},
        {QStringLiteral("chat-message-new"), [](Pen &d) { bubble(d); d.line(12, 8, 12, 13); d.line(9.5, 10.5, 14.5, 10.5); }},
        {QStringLiteral("edit-copy"), [](Pen &d) { d.rect(8, 8, 12, 12, 2); d.poly({{16, 8}, {16, 5}, {15, 4}, {5, 4}, {4, 5}, {4, 15}, {5, 16}, {8, 16}}); }},
        {QStringLiteral("edit-cut"), [](Pen &d) { d.circle(6.5, 17.5, 2.8); d.circle(17.5, 17.5, 2.8); d.line(8.5, 15.5, 17, 4); d.line(15.5, 15.5, 7, 4); }},
        {QStringLiteral("edit-paste"), [](Pen &d) {
            d.poly({{15, 5}, {18, 5}, {19, 6}, {19, 20}, {18, 21}, {6, 21}, {5, 20}, {5, 6}, {6, 5}, {9, 5}});
            d.rect(9, 3, 6, 4, 1.5);
        }},
        {QStringLiteral("edit-undo"), [](Pen &d) { d.poly({{8, 5}, {4, 9}, {8, 13}}); d.poly({{4, 9}, {15, 9}}); d.arc(15, 14, 5, -90, 180); d.line(15, 19, 9, 19); }},
        {QStringLiteral("edit-redo"), [](Pen &d) { d.poly({{16, 5}, {20, 9}, {16, 13}}); d.poly({{20, 9}, {9, 9}}); d.arc(9, 14, 5, 90, 180); d.line(9, 19, 15, 19); }},
        {QStringLiteral("edit-select-all"), [](Pen &d) {
            d.p.save();
            QPen dashed = d.p.pen();
            dashed.setDashPattern({2.0, 2.0});
            d.p.setPen(dashed);
            d.rect(4, 4, 16, 16, 2);
            d.p.restore();
            d.rect(8, 8, 8, 8, 1.5);
        }},
        {QStringLiteral("tab-close"), [](Pen &d) { d.line(7, 7, 17, 17); d.line(17, 7, 7, 17); }},
        {QStringLiteral("close-small"), [](Pen &d) { d.line(8, 8, 16, 16); d.line(16, 8, 8, 16); }},
        {QStringLiteral("chevron-down"), [](Pen &d) { d.poly({{7, 10}, {12, 15}, {17, 10}}); }},
        {QStringLiteral("chevron-right"), [](Pen &d) { d.poly({{10, 7}, {15, 12}, {10, 17}}); }},
        {QStringLiteral("check"), [](Pen &d) { d.poly({{6, 12.5}, {10, 16.5}, {18, 8}}); }},
        {QStringLiteral("system-run"), [](Pen &d) { d.closed({{13, 3}, {5, 13.5}, {11.5, 13.5}, {10.5, 21}, {19, 10}, {12.5, 10}, {13, 3}}); }},
        {QStringLiteral("tools-report-bug"), [](Pen &d) { d.poly({{3, 12}, {7, 12}, {9.5, 6}, {13.5, 18}, {16, 12}, {21, 12}}); }},
        {QStringLiteral("applications-development"), [](Pen &d) { d.poly({{8, 7}, {3, 12}, {8, 17}}); d.poly({{16, 7}, {21, 12}, {16, 17}}); d.line(13.5, 5, 10.5, 19); }},
        {QStringLiteral("drive-harddisk"), [](Pen &d) { d.arc(12, 15, 8.5, 0, 180); d.line(3.5, 15, 20.5, 15); d.line(12, 15, 16, 9.5); d.dot(12, 15, 1.4); }},
        {QStringLiteral("preferences-system"), [](Pen &d) {
            d.circle(12, 12, 3);
            for (int angle = 0; angle < 360; angle += 45) {
                d.p.save();
                d.p.translate(12, 12);
                d.p.rotate(angle);
                d.p.drawLine(QPointF(0, -6), QPointF(0, -8.8));
                d.p.restore();
            }
            d.circle(12, 12, 6);
        }},
        {QStringLiteral("go-home"), [](Pen &d) { d.poly({{3, 11}, {12, 3.5}, {21, 11}}); d.poly({{5.5, 9.5}, {5.5, 20}, {18.5, 20}, {18.5, 9.5}}); d.poly({{10, 20}, {10, 14}, {14, 14}, {14, 20}}); }},
        {QStringLiteral("utilities-system-monitor"), [](Pen &d) { d.line(5, 20, 5, 13); d.line(10, 20, 10, 6); d.line(15, 20, 15, 10); d.line(20, 20, 20, 4); }},
        {QStringLiteral("studio-skills"), [](Pen &d) {
            d.closed({{12, 3}, {14, 10}, {21, 12}, {14, 14}, {12, 21}, {10, 14}, {3, 12}, {10, 10}});
        }},
        {QStringLiteral("package-x-generic"), [](Pen &d) {
            d.closed({{12, 3}, {20, 7.5}, {20, 16.5}, {12, 21}, {4, 16.5}, {4, 7.5}});
            d.poly({{4, 7.5}, {12, 12}, {20, 7.5}});
            d.line(12, 12, 12, 21);
        }},
        {QStringLiteral("studio-mods"), [](Pen &d) {
            d.line(4, 7, 20, 7); d.line(4, 12, 20, 12); d.line(4, 17, 20, 17);
            d.dot(9, 7, 2); d.dot(15, 12, 2); d.dot(7, 17, 2);
        }},
        {QStringLiteral("studio-ci"), [](Pen &d) { d.circle(6, 5.5, 2.3); d.circle(6, 18.5, 2.3); d.circle(18, 8, 2.3); d.line(6, 8, 6, 16); d.poly({{18, 10.3}, {18, 12}, {16, 14}, {8, 16.5}}); }},
        {QStringLiteral("studio-deploy"), [](Pen &d) { d.poly({{7, 17}, {5, 17}, {3, 15}, {3, 12}, {5, 10}, {7, 10}, {8, 7}, {11, 5}, {15, 5}, {18, 8}, {18.5, 10}, {20, 11}, {21, 13.5}, {20, 16}, {17, 17}}); d.poly({{9.5, 14.5}, {12, 12}, {14.5, 14.5}}); d.line(12, 12, 12, 20); }},
        {QStringLiteral("document-edit"), [](Pen &d) {
            d.poly({{11, 20}, {5, 20}, {4, 19}, {4, 5}, {5, 4}, {14, 4}, {18, 8}, {18, 11}});
            d.poly({{13.5, 20.5}, {13.5, 18}, {19, 12.5}, {21.5, 15}, {16, 20.5}, {13.5, 20.5}});
        }},
        {QStringLiteral("studio-memory"), [](Pen &d) { d.poly({{6, 3}, {18, 3}, {18, 21}, {12, 16.5}, {6, 21}, {6, 3}}); }},
        {QStringLiteral("studio-brand"), [](Pen &d) { d.closed({{12, 3}, {20, 9}, {12, 21}, {4, 9}}); d.poly({{4, 9}, {20, 9}}); d.poly({{9, 3.8}, {12, 9}, {15, 3.8}}); d.line(12, 9, 12, 21); }},
        {QStringLiteral("studio-palette"), [](Pen &d) {
            d.p.drawPath([] { QPainterPath path; path.moveTo(12, 3); path.cubicTo(5, 3, 3, 8, 3, 12); path.cubicTo(3, 17, 7, 21, 12, 21);
                path.cubicTo(14, 21, 14, 19, 13, 18); path.cubicTo(12, 16.5, 13, 15, 15, 15); path.lineTo(17, 15); path.cubicTo(19.5, 15, 21, 13.5, 21, 11);
                path.cubicTo(21, 6.5, 17, 3, 12, 3); return path; }());
            d.dot(8, 10, 1.3); d.dot(12, 7, 1.3); d.dot(16, 9.5, 1.3);
        }},
        {QStringLiteral("list-add"), [](Pen &d) { d.line(12, 5, 12, 19); d.line(5, 12, 19, 12); }},
        {QStringLiteral("list-remove"), [](Pen &d) { d.line(5, 12, 19, 12); }},
        {QStringLiteral("go-up"), [](Pen &d) { d.line(12, 19, 12, 6); d.poly({{6.5, 11.5}, {12, 6}, {17.5, 11.5}}); }},
        {QStringLiteral("go-down"), [](Pen &d) { d.line(12, 5, 12, 18); d.poly({{6.5, 12.5}, {12, 18}, {17.5, 12.5}}); }},
        {QStringLiteral("git-commit"), [](Pen &d) { d.circle(12, 12, 3.6); d.line(3, 12, 8.4, 12); d.line(15.6, 12, 21, 12); }},
        {QStringLiteral("git-merge"), [](Pen &d) {
            d.circle(6, 5.5, 2.3); d.circle(6, 18.5, 2.3); d.circle(18, 13, 2.3);
            d.line(6, 7.8, 6, 16.2);
            d.poly({{6, 8}, {6, 10.5}, {8.5, 13}, {15.7, 13}});
        }},
        {QStringLiteral("git-pull"), [](Pen &d) { d.line(12, 4, 12, 15); d.poly({{7, 10.5}, {12, 15.5}, {17, 10.5}}); d.line(5, 20, 19, 20); }},
        {QStringLiteral("git-push"), [](Pen &d) { d.line(12, 20, 12, 9); d.poly({{7, 13.5}, {12, 8.5}, {17, 13.5}}); d.line(5, 4, 19, 4); }},
        {QStringLiteral("git-stash"), [](Pen &d) {
            d.rect(3, 4, 18, 5, 1.5);
            d.poly({{5, 9}, {5, 19}, {6, 20}, {18, 20}, {19, 19}, {19, 9}});
            d.line(10, 13, 14, 13);
        }},
        {QStringLiteral("check-circle"), [](Pen &d) { d.circle(12, 12, 9); d.poly({{8, 12.4}, {11, 15.4}, {16, 9.2}}); }},
        {QStringLiteral("studio-assistant"), [](Pen &d) {
            d.closed({{10, 5}, {11.8, 11.2}, {18, 13}, {11.8, 14.8}, {10, 21}, {8.2, 14.8}, {2, 13}, {8.2, 11.2}});
            d.line(19, 2.5, 19, 7.5); d.line(16.5, 5, 21.5, 5);
        }},
        {QStringLiteral("studio-advice"), [](Pen &d) {
            d.circle(12, 9.5, 5.5);
            d.poly({{9.5, 14.2}, {9.5, 17}, {14.5, 17}, {14.5, 14.2}});
            d.line(10, 20, 14, 20);
        }},
        {QStringLiteral("studio-git"), [](Pen &d) { d.circle(6, 6, 2.3); d.circle(6, 18, 2.3); d.circle(18, 12, 2.3); d.line(6, 8.3, 6, 15.7); d.poly({{8.2, 6.5}, {13, 6.5}, {16.3, 10.3}}); }},
    };
    return icons;
}

class StudioIconEngine final : public QIconEngine {
public:
    explicit StudioIconEngine(QString name) : m_name(std::move(name)) {}

    void paint(QPainter *painter, const QRect &rect, QIcon::Mode mode, QIcon::State state) override
    {
        const StudioTheme &theme = ThemeManager::current();
        QColor color = theme.textMuted;
        if (mode == QIcon::Disabled) color = theme.textFaint;
        else if (mode == QIcon::Active) color = theme.text;
        else if (mode == QIcon::Selected || state == QIcon::On) color = theme.accent;
        draw(*painter, rect, color);
    }

    QPixmap pixmap(const QSize &size, QIcon::Mode mode, QIcon::State state) override
    {
        QPixmap result(size);
        result.fill(Qt::transparent);
        QPainter painter(&result);
        paint(&painter, QRect(QPoint(0, 0), size), mode, state);
        return result;
    }

    QIconEngine *clone() const override { return new StudioIconEngine(m_name); }
    QString key() const override { return QStringLiteral("StudioIconEngine"); }

    void draw(QPainter &painter, const QRect &rect, const QColor &color) const
    {
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing);
        const qreal side = qMin(rect.width(), rect.height());
        painter.translate(rect.x() + (rect.width() - side) / 2.0, rect.y() + (rect.height() - side) / 2.0);
        painter.scale(side / 24.0, side / 24.0);
        QPen pen(color, 1.7, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
        painter.setPen(pen);
        painter.setBrush(Qt::NoBrush);
        Pen helper{painter};
        drawings().value(m_name)(helper);
        painter.restore();
    }

private:
    QString m_name;
};

} // namespace

bool hasStudioIcon(const QString &name)
{
    return drawings().contains(name);
}

QIcon studioIcon(const QString &name, QStyle::StandardPixmap fallback)
{
    if (hasStudioIcon(name)) return QIcon(new StudioIconEngine(name));
    return QIcon::fromTheme(name, QApplication::style()->standardIcon(fallback));
}

QPixmap renderStudioIcon(const QString &name, const QColor &color, int size)
{
    QPixmap result(size, size);
    result.fill(Qt::transparent);
    if (!hasStudioIcon(name)) return result;
    QPainter painter(&result);
    StudioIconEngine(name).draw(painter, result.rect(), color);
    return result;
}
