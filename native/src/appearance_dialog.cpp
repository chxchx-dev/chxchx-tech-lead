#include "appearance_dialog.hpp"
#include "studio_theme.hpp"

#include <QDialogButtonBox>
#include <QFontDatabase>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPushButton>
#include <QStyle>
#include <QVBoxLayout>

namespace {

QString span(const QColor &color, const QString &text, bool italic = false)
{
    return QStringLiteral("<span style='color:%1;%3'>%2</span>")
        .arg(color.name(), text.toHtmlEscaped(), italic ? QStringLiteral(" font-style:italic;") : QString());
}

// A few lines of Python coloured with the theme's syntax palette.
QString codePreview(const StudioTheme &t)
{
    const QString indent = QStringLiteral("&nbsp;&nbsp;&nbsp;&nbsp;");
    return QStringLiteral("<pre style='margin:0; line-height:140%;'>")
        + span(t.preprocessor, QStringLiteral("@cache")) + QStringLiteral("<br>")
        + span(t.keyword, QStringLiteral("def ")) + span(t.function, QStringLiteral("saludar"))
        + span(t.operatorColor, QStringLiteral("(")) + span(t.variable, QStringLiteral("nombre"))
        + span(t.operatorColor, QStringLiteral(": ")) + span(t.type, QStringLiteral("str"))
        + span(t.operatorColor, QStringLiteral(") -> ")) + span(t.type, QStringLiteral("str"))
        + span(t.operatorColor, QStringLiteral(":")) + QStringLiteral("<br>")
        + indent + span(t.comment, QStringLiteral("# Devuelve un saludo"), true) + QStringLiteral("<br>")
        + indent + span(t.keyword, QStringLiteral("return ")) + span(t.string, QStringLiteral("f\"Hola {nombre}\""))
        + span(t.operatorColor, QStringLiteral(" * ")) + span(t.number, QStringLiteral("2"))
        + QStringLiteral("</pre>");
}

QLabel *swatch(const QColor &color, QWidget *parent)
{
    auto *dot = new QLabel(parent);
    dot->setFixedSize(14, 14);
    dot->setStyleSheet(QStringLiteral("background-color: %1; border-radius: 7px;").arg(color.name()));
    return dot;
}

} // namespace

ThemeCard::ThemeCard(const StudioTheme &theme, QWidget *parent)
    : QFrame(parent)
    , m_themeId(theme.id)
{
    setObjectName(QStringLiteral("themeCard"));
    setCursor(Qt::PointingHandCursor);
    setFixedWidth(300);
    auto *column = new QVBoxLayout(this);
    column->setContentsMargins(12, 12, 12, 12);
    column->setSpacing(8);

    // The preview mimics a tiny window: title strip with the accent, then code.
    auto *preview = new QFrame(this);
    preview->setObjectName(QStringLiteral("themePreview"));
    preview->setStyleSheet(QStringLiteral(
        "#themePreview { background-color: %1; border: 1px solid %2; border-radius: 8px; }"
        "#themePreview QLabel { background: transparent; }")
        .arg(theme.editorBackground.name(), theme.border.name()));
    auto *previewColumn = new QVBoxLayout(preview);
    previewColumn->setContentsMargins(0, 0, 0, 8);
    previewColumn->setSpacing(6);
    auto *strip = new QFrame(preview);
    strip->setFixedHeight(22);
    strip->setStyleSheet(QStringLiteral("background-color: %1; border-top-left-radius: 8px; "
        "border-top-right-radius: 8px; border-bottom: 2px solid %2;").arg(theme.surface.name(), theme.accent.name()));
    auto *stripRow = new QHBoxLayout(strip);
    stripRow->setContentsMargins(8, 0, 8, 0);
    stripRow->setSpacing(5);
    for (const QColor &color : {theme.danger, theme.warning, theme.success}) stripRow->addWidget(swatch(color, strip));
    stripRow->addStretch(1);
    previewColumn->addWidget(strip);
    auto *code = new QLabel(codePreview(theme), preview);
    code->setTextFormat(Qt::RichText);
    code->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    code->setContentsMargins(10, 0, 10, 0);
    // Keep every line visible even when a long description takes space.
    code->setMinimumHeight(code->sizeHint().height());
    previewColumn->addWidget(code);
    column->addWidget(preview);

    auto *nameRow = new QHBoxLayout;
    auto *name = new QLabel(theme.name, this);
    name->setObjectName(QStringLiteral("themeName"));
    nameRow->addWidget(name);
    nameRow->addStretch(1);
    for (const QColor &color : {theme.accent, theme.keyword, theme.function, theme.string, theme.type})
        nameRow->addWidget(swatch(color, this));
    column->addLayout(nameRow);
    auto *description = new QLabel(theme.description, this);
    description->setObjectName(QStringLiteral("themeDescription"));
    description->setWordWrap(true);
    column->addWidget(description);
}

void ThemeCard::setSelected(bool selected)
{
    setProperty("selected", selected);
    style()->unpolish(this);
    style()->polish(this);
}

void ThemeCard::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) emit chosen(m_themeId);
    QFrame::mousePressEvent(event);
}

AppearanceDialog::AppearanceDialog(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle(QStringLiteral("Apariencia"));
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(20, 18, 20, 16);
    layout->setSpacing(12);
    auto *title = new QLabel(QStringLiteral("Elige un tema"), this);
    title->setObjectName(QStringLiteral("themeName"));
    title->setStyleSheet(QStringLiteral("font-size: 18px;"));
    auto *subtitle = new QLabel(QStringLiteral(
        "El tema colorea la interfaz, el editor, la terminal y el chat. El cambio se aplica al instante y se recuerda."), this);
    subtitle->setObjectName(QStringLiteral("pageIntro"));
    subtitle->setWordWrap(true);
    layout->addWidget(title);
    layout->addWidget(subtitle);

    auto *grid = new QGridLayout;
    grid->setSpacing(14);
    int index = 0;
    for (const StudioTheme &theme : studioThemes()) {
        auto *card = new ThemeCard(theme, this);
        card->setSelected(theme.id == ThemeManager::current().id);
        connect(card, &ThemeCard::chosen, this, [this](const QString &id) {
            ThemeManager::instance()->setTheme(id);
            for (ThemeCard *other : m_cards) other->setSelected(other->themeId() == id);
        });
        m_cards << card;
        grid->addWidget(card, index / 3, index % 3);
        ++index;
    }
    layout->addLayout(grid);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    buttons->button(QDialogButtonBox::Close)->setText(QStringLiteral("Listo"));
    buttons->button(QDialogButtonBox::Close)->setProperty("primary", true);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
    layout->addWidget(buttons);
}
