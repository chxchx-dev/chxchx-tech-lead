#include "agent_chat_cards.hpp"
#include "agent_chat_markdown.hpp"
#include "../studio_theme.hpp"

#include <QApplication>
#include <QClipboard>
#include <QFontDatabase>
#include <QHash>
#include <QHBoxLayout>
#include <QLabel>
#include <QPixmap>
#include <QPlainTextEdit>
#include <QStyle>
#include <QTimer>
#include <QToolButton>
#include <QVBoxLayout>

#include <functional>

namespace {

constexpr int AvatarSize = 28;
constexpr int Indent = AvatarSize + 10;
constexpr int MaxCodeLines = 18;
const QStringList SpinnerFrames = {QStringLiteral("⠋"), QStringLiteral("⠙"), QStringLiteral("⠹"),
    QStringLiteral("⠸"), QStringLiteral("⠼"), QStringLiteral("⠴"), QStringLiteral("⠦"), QStringLiteral("⠧"),
    QStringLiteral("⠇"), QStringLiteral("⠏")};

QLabel *richLabel(const QString &html, QWidget *parent)
{
    auto *label = new QLabel(html, parent);
    label->setTextFormat(Qt::RichText);
    label->setWordWrap(true);
    label->setOpenExternalLinks(true);
    label->setTextInteractionFlags(Qt::TextBrowserInteraction);
    label->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    return label;
}

QLabel *plainLabel(const QString &text, const QString &objectName, QWidget *parent)
{
    auto *label = new QLabel(text, parent);
    label->setObjectName(objectName);
    label->setTextFormat(Qt::PlainText);
    label->setWordWrap(true);
    label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    return label;
}

int monospaceHeight(const QPlainTextEdit *view, int lines)
{
    const int visible = qBound(1, lines, MaxCodeLines);
    // Line spacing plus document margins, stylesheet padding (6+6) and frame.
    return view->fontMetrics().lineSpacing() * visible + int(view->document()->documentMargin() * 2) + 16;
}

QPlainTextEdit *monospaceView(const QString &text, QWidget *parent)
{
    auto *view = new QPlainTextEdit(text, parent);
    view->setObjectName(QStringLiteral("chatMono"));
    view->setReadOnly(true);
    view->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    view->setFrameShape(QFrame::NoFrame);
    view->setLineWrapMode(QPlainTextEdit::NoWrap);
    const int lines = int(text.count(QLatin1Char('\n'))) + 1;
    view->setFixedHeight(monospaceHeight(view, lines));
    // Short snippets show whole; only long ones scroll.
    view->setVerticalScrollBarPolicy(lines > MaxCodeLines ? Qt::ScrollBarAsNeeded : Qt::ScrollBarAlwaysOff);
    return view;
}

QToolButton *copyButton(const QString &label, std::function<QString()> text, QWidget *parent)
{
    auto *button = new QToolButton(parent);
    button->setObjectName(QStringLiteral("chatGhost"));
    button->setText(label);
    button->setCursor(Qt::PointingHandCursor);
    QObject::connect(button, &QToolButton::clicked, button, [button, label, text = std::move(text)] {
        QApplication::clipboard()->setText(text());
        button->setText(QStringLiteral("Copiado ✓"));
        QTimer::singleShot(1500, button, [button, label] { button->setText(label); });
    });
    return button;
}

// Row with the avatar column on the left, so agent output lines up.
QHBoxLayout *agentRow(QWidget *card, const ChatIdentity &identity, bool continuation)
{
    auto *row = new QHBoxLayout(card);
    row->setContentsMargins(0, continuation ? 0 : 6, 0, 0);
    row->setSpacing(10);
    if (continuation) {
        row->addSpacing(AvatarSize);
    } else {
        row->addWidget(createChatAvatar(identity, AvatarSize, card), 0, Qt::AlignTop);
    }
    return row;
}

class UserCard final : public ChatEntryCard {
public:
    UserCard(const AgentChatModel::Entry &entry, QWidget *parent) : ChatEntryCard(parent)
    {
        auto *row = new QHBoxLayout(this);
        row->setContentsMargins(0, 10, 0, 2);
        row->addStretch(1);
        m_bubble = new QFrame(this);
        m_bubble->setObjectName(QStringLiteral("userBubble"));
        auto *column = new QVBoxLayout(m_bubble);
        column->setContentsMargins(14, 10, 14, 10);
        m_text = plainLabel(entry.text, QStringLiteral("userText"), m_bubble);
        column->addWidget(m_text);
        int naturalWidth = 0;
        for (const QString &line : entry.text.split(QLatin1Char('\n')))
            naturalWidth = qMax(naturalWidth, m_text->fontMetrics().horizontalAdvance(line));
        m_naturalWidth = naturalWidth + 30;
        if (!entry.images.isEmpty()) {
            auto *images = new QHBoxLayout;
            for (const QString &path : entry.images) {
                auto *thumb = new QLabel(m_bubble);
                thumb->setObjectName(QStringLiteral("chatThumb"));
                const QPixmap pixmap = QPixmap(path).scaledToHeight(96, Qt::SmoothTransformation);
                thumb->setPixmap(pixmap);
                m_imagesWidth += pixmap.width() + 8;
                thumb->setToolTip(path);
                images->addWidget(thumb);
            }
            images->addStretch(1);
            column->addLayout(images);
        }
        row->addWidget(m_bubble);
    }
    void setAvailableWidth(int width) override
    {
        // Wrapped labels report a narrow size hint, so size the bubble from its text.
        const int maximum = qMax(240, width * 4 / 5);
        m_bubble->setFixedWidth(qMin(maximum, qMax(m_naturalWidth, m_imagesWidth + 30)));
    }

private:
    QFrame *m_bubble = nullptr;
    QLabel *m_text = nullptr;
    int m_naturalWidth = 0;
    int m_imagesWidth = 0;
};

class AssistantCard final : public ChatEntryCard {
public:
    AssistantCard(const AgentChatModel::Entry &entry, const ChatIdentity &identity, bool continuation,
        QWidget *parent) : ChatEntryCard(parent)
    {
        auto *row = agentRow(this, identity, continuation);
        auto *column = new QVBoxLayout;
        column->setSpacing(6);
        if (!continuation) {
            auto *name = new QLabel(identity.name, this);
            name->setObjectName(QStringLiteral("agentName"));
            name->setStyleSheet(QStringLiteral("color: %1;").arg(identity.accent));
            column->addWidget(name);
        }
        for (const ChatSegment &segment : splitFencedCode(entry.text)) {
            if (segment.code) {
                column->addWidget(new ChatCodeBlock(segment.language, segment.text, this));
            } else {
                QLabel *prose = richLabel(chatMarkdownToHtml(segment.text), this);
                prose->setObjectName(QStringLiteral("agentText"));
                column->addWidget(prose);
            }
        }
        row->addLayout(column, 1);
    }
};

class ReasoningCard final : public ChatEntryCard {
public:
    ReasoningCard(const AgentChatModel::Entry &entry, QWidget *parent) : ChatEntryCard(parent)
    {
        auto *column = new QVBoxLayout(this);
        column->setContentsMargins(Indent, 0, 0, 0);
        auto *toggle = new QToolButton(this);
        toggle->setObjectName(QStringLiteral("chatGhost"));
        toggle->setText(QStringLiteral("💭 Razonamiento  ›"));
        toggle->setCursor(Qt::PointingHandCursor);
        QLabel *text = plainLabel(entry.text, QStringLiteral("reasoningText"), this);
        text->hide();
        connect(toggle, &QToolButton::clicked, this, [toggle, text] {
            text->setVisible(!text->isVisible());
            toggle->setText(text->isVisible() ? QStringLiteral("💭 Razonamiento  ⌄")
                                              : QStringLiteral("💭 Razonamiento  ›"));
        });
        column->addWidget(toggle, 0, Qt::AlignLeft);
        column->addWidget(text);
    }
};

class NoticeCard final : public ChatEntryCard {
public:
    NoticeCard(const AgentChatModel::Entry &entry, QWidget *parent) : ChatEntryCard(parent)
    {
        auto *row = new QHBoxLayout(this);
        row->setContentsMargins(0, 2, 0, 2);
        QLabel *text = plainLabel(QStringLiteral("ℹ  ") + entry.text, QStringLiteral("noticeText"), this);
        text->setAlignment(Qt::AlignCenter);
        row->addWidget(text);
    }
};

class ErrorCard final : public ChatEntryCard {
public:
    ErrorCard(const AgentChatModel::Entry &entry, QWidget *parent) : ChatEntryCard(parent)
    {
        setObjectName(QStringLiteral("errorCard"));
        auto *column = new QVBoxLayout(this);
        column->setContentsMargins(14, 12, 14, 12);
        auto *title = new QLabel(QStringLiteral("⚠  Algo salió mal"), this);
        title->setObjectName(QStringLiteral("errorTitle"));
        column->addWidget(title);
        QStringList prose;
        QString command;
        for (const QString &line : entry.text.split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();
            if (command.isEmpty() && (trimmed.startsWith(QStringLiteral("curl ")) || trimmed.startsWith(QStringLiteral("npm "))))
                command = trimmed;
            else
                prose << line;
        }
        column->addWidget(plainLabel(prose.join(QLatin1Char('\n')).trimmed(), QStringLiteral("errorText"), this));
        if (!command.isEmpty()) {
            // Install commands get their own copyable box.
            auto *box = new QFrame(this);
            box->setObjectName(QStringLiteral("commandBox"));
            auto *boxRow = new QHBoxLayout(box);
            boxRow->setContentsMargins(10, 6, 6, 6);
            auto *code = plainLabel(command, QStringLiteral("commandText"), box);
            code->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
            boxRow->addWidget(code, 1);
            boxRow->addWidget(copyButton(QStringLiteral("Copiar comando"), [command] { return command; }, box));
            column->addWidget(box);
        }
    }
};

} // namespace

ChatIdentity chatIdentityFor(const QString &provider, const QString &fallbackName)
{
    if (provider == QStringLiteral("Codex")) return {QStringLiteral("Codex"), QStringLiteral("</>"), QStringLiteral("#10a37f")};
    if (provider == QStringLiteral("Claude")) return {QStringLiteral("Claude"), QStringLiteral("✳"), QStringLiteral("#d97757")};
    return {fallbackName, QStringLiteral("◆"), ThemeManager::current().accent.name()};
}

QLabel *createChatAvatar(const ChatIdentity &identity, int size, QWidget *parent)
{
    auto *avatar = new QLabel(identity.glyph, parent);
    avatar->setAlignment(Qt::AlignCenter);
    avatar->setFixedSize(size, size);
    avatar->setStyleSheet(QStringLiteral(
        "background-color: %1; color: #ffffff; border-radius: %2px; font-weight: 700; font-size: %3px;")
        .arg(identity.accent).arg(size / 2).arg(qMax(10, size * 2 / 5)));
    return avatar;
}

ChatEntryCard *createChatEntryCard(const AgentChatModel::Entry &entry, const ChatIdentity &identity,
    bool continuation, QWidget *parent)
{
    switch (entry.kind) {
    case AgentChatModel::Kind::User: return new UserCard(entry, parent);
    case AgentChatModel::Kind::Assistant: return new AssistantCard(entry, identity, continuation, parent);
    case AgentChatModel::Kind::Reasoning: return new ReasoningCard(entry, parent);
    case AgentChatModel::Kind::Tool: return new ChatToolCard(entry, parent);
    case AgentChatModel::Kind::Notice: return new NoticeCard(entry, parent);
    case AgentChatModel::Kind::Error: return new ErrorCard(entry, parent);
    }
    return new NoticeCard(entry, parent);
}

ChatCodeBlock::ChatCodeBlock(const QString &language, const QString &code, QWidget *parent)
    : QFrame(parent)
    , m_code(code)
{
    setObjectName(QStringLiteral("codeBlock"));
    auto *column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);
    column->setSpacing(0);
    auto *header = new QFrame(this);
    header->setObjectName(QStringLiteral("codeHeader"));
    auto *headerRow = new QHBoxLayout(header);
    headerRow->setContentsMargins(12, 4, 6, 4);
    auto *lang = new QLabel((language.isEmpty() ? QStringLiteral("código") : language).toUpper(), header);
    lang->setObjectName(QStringLiteral("codeLanguage"));
    headerRow->addWidget(lang);
    headerRow->addStretch(1);
    headerRow->addWidget(copyButton(QStringLiteral("Copiar"), [this] { return m_code; }, header));
    column->addWidget(header);
    column->addWidget(monospaceView(code, this));
}

ChatToolCard::ChatToolCard(const AgentChatModel::Entry &entry, QWidget *parent)
    : ChatEntryCard(parent)
{
    auto *outer = new QVBoxLayout(this);
    outer->setContentsMargins(Indent, 1, 0, 1);
    m_frame = new QFrame(this);
    m_frame->setObjectName(QStringLiteral("toolCard"));
    QFrame *frame = m_frame;
    auto *column = new QVBoxLayout(frame);
    column->setContentsMargins(10, 6, 8, 6);
    column->setSpacing(4);
    auto *header = new QHBoxLayout;
    m_badge = new QLabel(frame);
    m_badge->setFixedWidth(16);
    m_title = new QLabel(frame);
    m_title->setObjectName(QStringLiteral("toolTitle"));
    m_title->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    m_title->setTextFormat(Qt::PlainText);
    m_title->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    m_meta = new QLabel(frame);
    m_meta->setObjectName(QStringLiteral("toolMeta"));
    m_toggle = new QToolButton(frame);
    m_toggle->setObjectName(QStringLiteral("chatGhost"));
    m_toggle->setCursor(Qt::PointingHandCursor);
    header->addWidget(m_badge);
    header->addWidget(m_title, 1);
    header->addWidget(m_meta);
    header->addWidget(m_toggle);
    column->addLayout(header);
    m_output = monospaceView(QString(), frame);
    m_output->setMaximumHeight(220);
    m_output->hide();
    column->addWidget(m_output);
    outer->addWidget(frame);
    connect(m_toggle, &QToolButton::clicked, this, [this] { setExpanded(!isExpanded()); });
    refresh(entry);
}

void ChatToolCard::refresh(const AgentChatModel::Entry &entry)
{
    static const QHash<QString, QString> icons = {{QStringLiteral("command"), QStringLiteral("$")},
        {QStringLiteral("file_change"), QStringLiteral("✎")}, {QStringLiteral("search"), QStringLiteral("⌕")},
        {QStringLiteral("todo"), QStringLiteral("☐")}, {QStringLiteral("mcp"), QStringLiteral("⚙")}};
    m_status = entry.status;
    m_title->setText(QStringLiteral("%1  %2").arg(icons.value(entry.toolKind, QStringLiteral("⚙")), entry.text));
    m_title->setToolTip(entry.text);
    const QString output = entry.output.trimmed();
    m_output->setPlainText(output);
    const int lines = output.isEmpty() ? 0 : int(output.count(QLatin1Char('\n'))) + 1;
    m_output->setFixedHeight(qMin(220, monospaceHeight(m_output, qMax(1, lines))));
    m_output->setVerticalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    m_meta->setText(lines > 0 ? QStringLiteral("%1 línea%2").arg(lines).arg(lines == 1 ? QString() : QStringLiteral("s")) : QString());
    m_toggle->setVisible(lines > 0);
    m_toggle->setText(isExpanded() ? QStringLiteral("⌄") : QStringLiteral("›"));
    // The stylesheet colours the card edge by status.
    m_frame->setProperty("status", m_status == QStringLiteral("failed") ? QStringLiteral("failed")
        : (m_status == QStringLiteral("completed") ? QStringLiteral("completed") : QStringLiteral("running")));
    m_frame->style()->unpolish(m_frame);
    m_frame->style()->polish(m_frame);
    tick(0);
}

void ChatToolCard::tick(int frame)
{
    const StudioTheme &theme = ThemeManager::current();
    if (m_status == QStringLiteral("completed")) {
        m_badge->setText(QStringLiteral("<span style='color:%1;'>✓</span>").arg(theme.success.name()));
    } else if (m_status == QStringLiteral("failed")) {
        m_badge->setText(QStringLiteral("<span style='color:%1;'>✗</span>").arg(theme.danger.name()));
    } else {
        m_badge->setText(QStringLiteral("<span style='color:%1;'>%2</span>")
            .arg(theme.accent.name(), SpinnerFrames.at(frame % SpinnerFrames.size())));
    }
}

void ChatToolCard::setExpanded(bool expanded)
{
    m_expanded = expanded;
    m_output->setVisible(expanded);
    m_toggle->setText(expanded ? QStringLiteral("⌄") : QStringLiteral("›"));
}
