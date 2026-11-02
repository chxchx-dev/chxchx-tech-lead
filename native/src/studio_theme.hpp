#pragma once

#include <QColor>
#include <QList>
#include <QObject>
#include <QString>
#include <QTemporaryDir>

class QApplication;

// Every colour Studio paints with: interface, editor chrome and syntax roles.
// Widgets, icons, the code editor and the agent chat all read from here so a
// theme switch restyles the whole app at once.
struct StudioTheme {
    QString id;
    QString name;
    QString description;
    bool dark = true;

    QColor background;
    QColor surface;
    QColor surfaceRaised;
    QColor surfaceHover;
    QColor border;
    QColor borderStrong;
    QColor text;
    QColor textMuted;
    QColor textFaint;
    QColor accent;
    QColor accentStrong;
    QColor onAccent;
    QColor accentSoft;
    QColor selectedText;
    QColor success;
    QColor warning;
    QColor danger;

    QColor editorBackground;
    QColor gutterText;
    QColor gutterActive;
    QColor caret;
    QColor caretLine;
    QColor selection;
    QColor indentGuide;

    QColor comment;
    QColor keyword;
    QColor type;
    QColor string;
    QColor number;
    QColor function;
    QColor constant;
    QColor operatorColor;
    QColor preprocessor;
    QColor tag;
    QColor attribute;
    QColor variable;
};

const QList<StudioTheme> &studioThemes();
const StudioTheme &studioThemeById(const QString &id);

// Owns the active theme, persists the choice and restyles the application.
class ThemeManager final : public QObject {
    Q_OBJECT
public:
    static ThemeManager *instance();
    static const StudioTheme &current() { return instance()->m_theme; }

    void setTheme(const QString &id);
    // Replaces @token@ placeholders (e.g. @accent@) with the theme's colours.
    static QString expand(QString styleSheet, const StudioTheme &theme);
    // Path of a themed PNG (chevron, close, check) usable from stylesheets.
    QString chromeImage(const QString &name) const;

signals:
    void themeChanged(const StudioTheme &theme);

private:
    ThemeManager();
    void apply();

    StudioTheme m_theme;
    QTemporaryDir m_chrome;
};

// Loads the saved theme (default "medianoche") and styles the application.
void applyStudioTheme(QApplication &application);
QString studioStyleSheet(const StudioTheme &theme);
