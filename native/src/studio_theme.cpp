#include "studio_theme.hpp"
#include "studio_icons.hpp"

#include <QApplication>
#include <QFile>
#include <QPalette>
#include <QRegularExpression>
#include <QSettings>

namespace {

const QString ThemeSetting = QStringLiteral("appearance/theme");
const QStringList ChromeImages = {QStringLiteral("chevron-down"), QStringLiteral("chevron-right"),
    QStringLiteral("close-small"), QStringLiteral("check"), QStringLiteral("window-float")};

QPalette paletteFor(const StudioTheme &theme)
{
    QPalette palette;
    palette.setColor(QPalette::Window, theme.background);
    palette.setColor(QPalette::WindowText, theme.text);
    palette.setColor(QPalette::Base, theme.surface);
    palette.setColor(QPalette::AlternateBase, theme.surfaceRaised);
    palette.setColor(QPalette::Text, theme.text);
    palette.setColor(QPalette::PlaceholderText, theme.textFaint);
    palette.setColor(QPalette::Button, theme.surfaceRaised);
    palette.setColor(QPalette::ButtonText, theme.text);
    palette.setColor(QPalette::Highlight, theme.accentStrong);
    palette.setColor(QPalette::HighlightedText, theme.onAccent);
    palette.setColor(QPalette::ToolTipBase, theme.surfaceRaised);
    palette.setColor(QPalette::ToolTipText, theme.text);
    palette.setColor(QPalette::Link, theme.accent);
    palette.setColor(QPalette::Disabled, QPalette::Text, theme.textFaint);
    palette.setColor(QPalette::Disabled, QPalette::ButtonText, theme.textFaint);
    palette.setColor(QPalette::Disabled, QPalette::WindowText, theme.textFaint);
    return palette;
}

} // namespace

ThemeManager *ThemeManager::instance()
{
    static ThemeManager *manager = new ThemeManager;
    return manager;
}

ThemeManager::ThemeManager()
    : m_theme(studioThemeById(QSettings().value(ThemeSetting).toString()))
{
}

void ThemeManager::setTheme(const QString &id)
{
    m_theme = studioThemeById(id);
    QSettings().setValue(ThemeSetting, m_theme.id);
    apply();
    emit themeChanged(m_theme);
}

QString ThemeManager::chromeImage(const QString &name) const
{
    // One file per theme: Qt caches stylesheet images by path.
    return m_chrome.filePath(QStringLiteral("%1-%2.png").arg(name, m_theme.id));
}

void ThemeManager::apply()
{
    if (m_chrome.isValid()) {
        for (const QString &name : ChromeImages) {
            const QString path = chromeImage(name);
            if (!QFile::exists(path)) renderStudioIcon(name, m_theme.textMuted, 32).save(path);
        }
    }
    if (auto *application = qobject_cast<QApplication *>(QCoreApplication::instance())) {
        application->setPalette(paletteFor(m_theme));
        application->setStyleSheet(studioStyleSheet(m_theme));
    }
}

QString ThemeManager::expand(QString styleSheet, const StudioTheme &theme)
{
    const std::pair<const char *, QColor> tokens[] = {
        {"background", theme.background}, {"surface", theme.surface}, {"surfaceRaised", theme.surfaceRaised},
        {"surfaceHover", theme.surfaceHover}, {"border", theme.border}, {"borderStrong", theme.borderStrong},
        {"text", theme.text}, {"textMuted", theme.textMuted}, {"textFaint", theme.textFaint},
        {"accent", theme.accent}, {"accentStrong", theme.accentStrong}, {"onAccent", theme.onAccent},
        {"accentSoft", theme.accentSoft}, {"selectedText", theme.selectedText}, {"success", theme.success},
        {"warning", theme.warning}, {"danger", theme.danger}, {"editorBackground", theme.editorBackground},
        {"caretLine", theme.caretLine}, {"selection", theme.selection}, {"comment", theme.comment},
        {"keyword", theme.keyword}, {"string", theme.string}, {"function", theme.function}, {"type", theme.type},
    };
    // Soft variants for error surfaces, blended over the background.
    const auto blend = [&theme](const QColor &color, qreal amount) {
        return QColor::fromRgbF(theme.background.redF() + (color.redF() - theme.background.redF()) * amount,
            theme.background.greenF() + (color.greenF() - theme.background.greenF()) * amount,
            theme.background.blueF() + (color.blueF() - theme.background.blueF()) * amount);
    };
    styleSheet.replace(QStringLiteral("@dangerSoft@"), blend(theme.danger, 0.14).name());
    styleSheet.replace(QStringLiteral("@dangerBorder@"), blend(theme.danger, 0.4).name());
    for (const auto &[name, color] : tokens) {
        styleSheet.replace(QStringLiteral("@%1@").arg(QString::fromLatin1(name)), color.name());
    }
    static const QRegularExpression image(QStringLiteral("@img:([a-z-]+)@"));
    auto match = image.match(styleSheet);
    while (match.hasMatch()) {
        styleSheet.replace(match.capturedStart(), match.capturedLength(),
            instance()->chromeImage(match.captured(1)));
        match = image.match(styleSheet);
    }
    return styleSheet;
}

void applyStudioTheme(QApplication &application)
{
    Q_UNUSED(application)
    ThemeManager::instance()->setTheme(ThemeManager::current().id);
}
