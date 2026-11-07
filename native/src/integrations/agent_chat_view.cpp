#include "agent_chat_view.hpp"
#include "agent_chat_permission.hpp"
#include "agent_chat_question.hpp"
#include "../responsive_grid.hpp"
#include "../studio_theme.hpp"

#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QResizeEvent>
#include <QScrollBar>
#include <QTimer>
#include <QVBoxLayout>

namespace {

constexpr int AnimationMs = 90;

bool continuesAgentTurn(const QList<AgentChatModel::Entry> &entries, qsizetype index)
{
    for (qsizetype previous = index - 1; previous >= 0; --previous) {
        const auto kind = entries.at(previous).kind;
        if (kind == AgentChatModel::Kind::Assistant) return true;
        if (kind != AgentChatModel::Kind::Tool && kind != AgentChatModel::Kind::Reasoning) return false;
    }
    return false;
}

} // namespace

AgentChatView::AgentChatView(QWidget *parent)
    : QScrollArea(parent)
    , m_identity(chatIdentityFor(QString(), QStringLiteral("Agente")))
{
    setObjectName(QStringLiteral("chatView"));
    setWidgetResizable(true);
    setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    setFrameShape(QFrame::NoFrame);
    m_content = new QWidget(this);
    m_content->setObjectName(QStringLiteral("chatContent"));
    m_layout = new QVBoxLayout(m_content);
    m_layout->setContentsMargins(18, 12, 18, 12);
    m_layout->setSpacing(6);

    m_typing = new QWidget(m_content);
    auto *typingRow = new QHBoxLayout(m_typing);
    typingRow->setContentsMargins(0, 6, 0, 0);
    typingRow->setSpacing(10);
    m_typingAvatar = createChatAvatar(m_identity, 28, m_typing);
    m_typingLabel = new QLabel(m_typing);
    m_typingLabel->setObjectName(QStringLiteral("typingLabel"));
    typingRow->addWidget(m_typingAvatar);
    typingRow->addWidget(m_typingLabel, 1);
    m_typing->hide();
    m_layout->addWidget(m_typing);
    m_layout->addStretch(1);
    setWidget(m_content);

    m_animation = new QTimer(this);
    m_animation->setInterval(AnimationMs);
    connect(m_animation, &QTimer::timeout, this, &AgentChatView::animate);
    QScrollBar *scroll = verticalScrollBar();
    connect(scroll, &QScrollBar::valueChanged, this, [this, scroll](int value) {
        m_stickToBottom = value >= scroll->maximum() - 24;
    });
    connect(scroll, &QScrollBar::rangeChanged, this, [this, scroll](int, int maximum) {
        if (m_stickToBottom) scroll->setValue(maximum);
    });
    rebuildWelcome();
}

void AgentChatView::setIdentity(const ChatIdentity &identity)
{
    m_identity = identity;
    auto *avatar = createChatAvatar(identity, 28, m_typing);
    m_typing->layout()->replaceWidget(m_typingAvatar, avatar);
    delete m_typingAvatar;
    m_typingAvatar = avatar;
    if (m_cards.isEmpty()) rebuildWelcome();
}

void AgentChatView::sync(const AgentChatModel &model)
{
    const auto &entries = model.entries();
    if (entries.size() < m_cards.size()) clearCards();
    m_welcome->setVisible(entries.isEmpty());
    for (qsizetype index = 0; index < entries.size(); ++index) {
        const auto &entry = entries.at(index);
        if (index < m_cards.size()) {
            if (m_revisions.at(index) != entry.revision) {
                m_cards.at(index)->refresh(entry);
                m_revisions[index] = entry.revision;
            }
            continue;
        }
        ChatEntryCard *card = createChatEntryCard(entry, m_identity, continuesAgentTurn(entries, index), m_content);
        card->setAvailableWidth(viewport()->width());
        if (auto *permission = qobject_cast<ChatPermissionCard *>(card)) {
            connect(permission, &ChatPermissionCard::choiceChosen, this,
                [this, index](int choice) { emit permissionAnswered(int(index), choice); });
        }
        if (auto *question = qobject_cast<ChatQuestionCard *>(card)) {
            connect(question, &ChatQuestionCard::answered, this,
                [this, index](const QJsonObject &answers) { emit questionAnswered(int(index), answers); });
        }
        // Cards go above the typing indicator and the trailing stretch.
        m_layout->insertWidget(m_layout->indexOf(m_typing), card);
        m_cards.append(card);
        m_revisions.append(entry.revision);
        if (entry.kind == AgentChatModel::Kind::User) m_stickToBottom = true;
    }
}

void AgentChatView::setBusy(bool busy, const QString &label)
{
    m_busyLabel = label.isEmpty() ? QStringLiteral("%1 está trabajando").arg(m_identity.name) : label;
    m_typing->setVisible(busy);
    if (busy) {
        m_stickToBottom = true;
        m_animation->start();
        animate();
    } else {
        m_animation->stop();
        for (ChatEntryCard *card : m_cards) card->tick(0);
    }
}

void AgentChatView::resizeEvent(QResizeEvent *event)
{
    QScrollArea::resizeEvent(event);
    for (ChatEntryCard *card : m_cards) card->setAvailableWidth(viewport()->width());
}

void AgentChatView::clearCards()
{
    qDeleteAll(m_cards);
    m_cards.clear();
    m_revisions.clear();
    m_stickToBottom = true;
}

void AgentChatView::rebuildWelcome()
{
    delete m_welcome;
    m_welcome = new QWidget(m_content);
    m_welcome->setObjectName(QStringLiteral("chatWelcome"));
    auto *column = new QVBoxLayout(m_welcome);
    column->setContentsMargins(0, 48, 0, 24);
    column->setSpacing(10);
    column->addWidget(createChatAvatar(m_identity, 56, m_welcome), 0, Qt::AlignHCenter);
    auto *title = new QLabel(QStringLiteral("¿En qué trabajamos hoy?"), m_welcome);
    title->setObjectName(QStringLiteral("welcomeTitle"));
    title->setAlignment(Qt::AlignCenter);
    auto *subtitle = new QLabel(QStringLiteral(
        "%1 recibe el contexto del proyecto: estado, handoff, memoria y últimas sesiones.").arg(m_identity.name),
        m_welcome);
    subtitle->setObjectName(QStringLiteral("welcomeSubtitle"));
    subtitle->setAlignment(Qt::AlignCenter);
    subtitle->setWordWrap(true);
    column->addWidget(title);
    column->addWidget(subtitle);
    column->addSpacing(8);
    const QStringList suggestions = {
        QStringLiteral("Explícame la arquitectura de este proyecto"),
        QStringLiteral("Continúa con el pendiente del handoff"),
        QStringLiteral("Revisa los cambios sin commitear y busca bugs"),
        QStringLiteral("Propón tests para la parte más crítica"),
    };
    // Two columns when there is room, one in narrow split panels.
    auto *grid = new ResponsiveGrid(240, 2, m_welcome);
    grid->setMaximumWidth(620);
    for (const QString &suggestion : suggestions) {
        auto *chip = new QPushButton(suggestion);
        chip->setObjectName(QStringLiteral("suggestionChip"));
        chip->setCursor(Qt::PointingHandCursor);
        connect(chip, &QPushButton::clicked, this, [this, suggestion] { emit suggestionChosen(suggestion); });
        grid->addWidget(chip);
    }
    // Stretches centre the grid; an alignment flag would shrink it to its hint.
    auto *centered = new QHBoxLayout;
    centered->addStretch(1);
    centered->addWidget(grid, 10);
    centered->addStretch(1);
    column->addLayout(centered);
    m_layout->insertWidget(0, m_welcome);
    m_welcome->setVisible(m_cards.isEmpty());
}

void AgentChatView::animate()
{
    ++m_frame;
    static const QStringList dots = {QStringLiteral("●○○"), QStringLiteral("○●○"), QStringLiteral("○○●")};
    m_typingLabel->setText(QStringLiteral("%1  <span style='color:%2;'>%3</span>")
        .arg(m_busyLabel.toHtmlEscaped(), ThemeManager::current().accent.name(), dots.at((m_frame / 4) % dots.size())));
    for (ChatEntryCard *card : m_cards) card->tick(m_frame);
}
