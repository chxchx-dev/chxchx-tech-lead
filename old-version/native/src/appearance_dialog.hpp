#pragma once

#include <QDialog>
#include <QFrame>
#include <QList>

struct StudioTheme;

// Clickable card with a live preview of a theme's interface and code colours.
class ThemeCard final : public QFrame {
    Q_OBJECT
public:
    ThemeCard(const StudioTheme &theme, QWidget *parent = nullptr);
    QString themeId() const { return m_themeId; }
    void setSelected(bool selected);

signals:
    void chosen(const QString &themeId);

protected:
    void mousePressEvent(QMouseEvent *event) override;

private:
    QString m_themeId;
};

// Theme picker: choosing a card restyles Studio immediately and remembers it.
class AppearanceDialog final : public QDialog {
    Q_OBJECT
public:
    explicit AppearanceDialog(QWidget *parent = nullptr);

private:
    QList<ThemeCard *> m_cards;
};
