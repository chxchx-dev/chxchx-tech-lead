#include "file_icon_provider.hpp"
#include "studio_theme.hpp"

#include <QFileInfo>
#include <QHash>
#include <QIconEngine>
#include <QPainter>
#include <QSet>

namespace {

QColor colorForKind(const QString &kind, const StudioTheme &theme)
{
    if (kind == QStringLiteral("folder")) return theme.accent;
    if (kind == QStringLiteral("code")) return theme.function;
    if (kind == QStringLiteral("doc")) return theme.string;
    if (kind == QStringLiteral("config")) return theme.constant;
    if (kind == QStringLiteral("style")) return theme.keyword;
    if (kind == QStringLiteral("image")) return theme.number;
    if (kind == QStringLiteral("build")) return theme.preprocessor;
    if (kind == QStringLiteral("dotfile")) return theme.textFaint;
    return theme.textMuted;
}

class FileIconEngine final : public QIconEngine {
public:
    explicit FileIconEngine(QString kind) : m_kind(std::move(kind)) {}

    void paint(QPainter *painter, const QRect &rect, QIcon::Mode, QIcon::State) override
    {
        painter->save();
        painter->setRenderHint(QPainter::Antialiasing);
        const qreal side = qMin(rect.width(), rect.height());
        painter->translate(rect.x() + (rect.width() - side) / 2.0, rect.y() + (rect.height() - side) / 2.0);
        painter->scale(side / 24.0, side / 24.0);
        const QColor color = colorForKind(m_kind, ThemeManager::current());
        painter->setPen(QPen(color, 1.7, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        if (m_kind == QStringLiteral("folder")) {
            QColor fill = color;
            fill.setAlphaF(0.18);
            painter->setBrush(fill);
            const QPointF folder[] = {{3, 7}, {3, 18}, {4, 19}, {20, 19}, {21, 18}, {21, 9}, {20, 8}, {12, 8}, {10, 5},
                {4, 5}, {3, 6}};
            painter->drawPolygon(folder, 11);
        } else {
            const QPointF page[] = {{14, 3}, {6, 3}, {5, 4}, {5, 20}, {6, 21}, {18, 21}, {19, 20}, {19, 8}, {14, 3}};
            painter->drawPolyline(page, 9);
            const QPointF fold[] = {{14, 3}, {14, 8}, {19, 8}};
            painter->drawPolyline(fold, 3);
            drawGlyph(*painter);
        }
        painter->restore();
    }

    QPixmap pixmap(const QSize &size, QIcon::Mode mode, QIcon::State state) override
    {
        QPixmap result(size);
        result.fill(Qt::transparent);
        QPainter painter(&result);
        paint(&painter, QRect(QPoint(0, 0), size), mode, state);
        return result;
    }

    QIconEngine *clone() const override { return new FileIconEngine(m_kind); }

private:
    // A small mark inside the page tells the kinds apart even without colour.
    void drawGlyph(QPainter &p) const
    {
        if (m_kind == QStringLiteral("code")) {
            const QPointF left[] = {{10, 12}, {8, 14.5}, {10, 17}};
            const QPointF right[] = {{14, 12}, {16, 14.5}, {14, 17}};
            p.drawPolyline(left, 3);
            p.drawPolyline(right, 3);
        } else if (m_kind == QStringLiteral("doc")) {
            p.drawLine(QPointF(8.5, 12.5), QPointF(15.5, 12.5));
            p.drawLine(QPointF(8.5, 16), QPointF(13.5, 16));
        } else if (m_kind == QStringLiteral("config")) {
            p.drawEllipse(QPointF(12, 14.5), 2.6, 2.6);
        } else if (m_kind == QStringLiteral("style")) {
            p.drawLine(QPointF(10, 11.5), QPointF(9.5, 17.5));
            p.drawLine(QPointF(14.5, 11.5), QPointF(14, 17.5));
            p.drawLine(QPointF(8, 13.5), QPointF(16, 13.5));
            p.drawLine(QPointF(8, 15.8), QPointF(16, 15.8));
        } else if (m_kind == QStringLiteral("image")) {
            const QPointF mountain[] = {{7.5, 18}, {11, 13.5}, {13.5, 16}, {15, 14.5}, {16.5, 18}};
            p.drawPolyline(mountain, 5);
        } else if (m_kind == QStringLiteral("build")) {
            p.drawLine(QPointF(9, 17.5), QPointF(14, 12.5));
            p.drawLine(QPointF(12.5, 11), QPointF(15.5, 14));
        }
    }

    QString m_kind;
};

} // namespace

QString studioFileKind(const QString &fileName, bool isDir)
{
    if (isDir) return QStringLiteral("folder");
    static const QSet<QString> buildNames = {QStringLiteral("CMakeLists.txt"), QStringLiteral("Makefile"),
        QStringLiteral("Dockerfile"), QStringLiteral("meson.build"), QStringLiteral("Justfile")};
    if (buildNames.contains(fileName)) return QStringLiteral("build");
    static const QHash<QString, QString> bySuffix = [] {
        QHash<QString, QString> table;
        for (const char *suffix : {"py", "pyi", "c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx", "js", "jsx", "mjs",
                 "ts", "tsx", "java", "cs", "kt", "kts", "go", "rs", "sh", "bash", "zsh", "ps1", "swift", "rb", "php",
                 "sql", "vue"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("code"));
        for (const char *suffix : {"md", "markdown", "txt", "rst", "adoc", "pdf"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("doc"));
        for (const char *suffix : {"json", "yaml", "yml", "toml", "ini", "cfg", "conf", "lock", "xml", "env", "qrc",
                 "ui", "properties", "editorconfig"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("config"));
        for (const char *suffix : {"css", "scss", "sass", "less", "qss", "html", "htm"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("style"));
        for (const char *suffix : {"png", "jpg", "jpeg", "gif", "svg", "ico", "webp", "bmp"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("image"));
        for (const char *suffix : {"cmake", "mk", "nsi", "spec"})
            table.insert(QString::fromLatin1(suffix), QStringLiteral("build"));
        return table;
    }();
    const QString suffix = QFileInfo(fileName).suffix().toLower();
    if (bySuffix.contains(suffix)) return bySuffix.value(suffix);
    if (fileName.startsWith(QLatin1Char('.'))) return QStringLiteral("dotfile");
    return QStringLiteral("file");
}

QIcon StudioFileIconProvider::icon(IconType type) const
{
    return QIcon(new FileIconEngine(type == Folder ? QStringLiteral("folder") : QStringLiteral("file")));
}

QIcon StudioFileIconProvider::icon(const QFileInfo &info) const
{
    return QIcon(new FileIconEngine(studioFileKind(info.fileName(), info.isDir())));
}
