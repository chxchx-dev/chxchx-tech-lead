#include "agent_chat_question.hpp"

#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

namespace {

constexpr int Indent = 38;

QLabel *label(const QString &text, const QString &objectName, QWidget *parent)
{
    auto *item = new QLabel(text, parent);
    item->setObjectName(objectName);
    item->setTextFormat(Qt::PlainText);
    item->setWordWrap(true);
    return item;
}

} // namespace

ChatQuestionCard::ChatQuestionCard(const AgentChatModel::Entry &entry, QWidget *parent)
    : ChatEntryCard(parent)
{
    auto *outer = new QVBoxLayout(this);
    outer->setContentsMargins(Indent, 2, 0, 2);
    auto *frame = new QFrame(this);
    frame->setObjectName(QStringLiteral("questionCard"));
    auto *column = new QVBoxLayout(frame);
    column->setContentsMargins(12, 10, 12, 10);
    column->setSpacing(8);
    column->addWidget(label(QStringLiteral("💬  %1 te pregunta").arg(QStringLiteral("El agente")), QStringLiteral("permissionTitle"), frame));
    m_body = new QWidget(frame);
    m_body->setObjectName(QStringLiteral("permissionButtons"));
    auto *body = new QVBoxLayout(m_body);
    body->setContentsMargins(0, 0, 0, 0);
    body->setSpacing(8);
    for (const auto &value : entry.questions) {
        const QJsonObject item = value.toObject();
        Block block;
        block.question = item.value(QStringLiteral("question")).toString();
        block.multi = item.value(QStringLiteral("multiSelect")).toBool();
        const QString header = item.value(QStringLiteral("header")).toString();
        if (!header.isEmpty()) body->addWidget(label(header.toUpper(), QStringLiteral("permissionDetail"), m_body));
        body->addWidget(label(block.question, QStringLiteral("permissionSummary"), m_body));
        auto *options = new QVBoxLayout;
        options->setSpacing(4);
        const int blockIndex = int(m_blocks.size());
        for (const auto &optionValue : item.value(QStringLiteral("options")).toArray()) {
            const QJsonObject option = optionValue.toObject();
            const QString text = option.value(QStringLiteral("label")).toString();
            const QString description = option.value(QStringLiteral("description")).toString();
            auto *button = new QPushButton(description.isEmpty() ? text : QStringLiteral("%1  ·  %2").arg(text, description), m_body);
            button->setObjectName(QStringLiteral("questionOption"));
            button->setCheckable(true);
            button->setCursor(Qt::PointingHandCursor);
            button->setProperty("answer", text);
            connect(button, &QPushButton::clicked, this, [this, blockIndex, text] { select(blockIndex, text); });
            options->addWidget(button);
            block.buttons << button;
        }
        block.free = new QLineEdit(m_body);
        block.free->setObjectName(QStringLiteral("questionFree"));
        block.free->setPlaceholderText(QStringLiteral("Otra respuesta…"));
        connect(block.free, &QLineEdit::returnPressed, this, [this, blockIndex] {
            const QString text = m_blocks.at(blockIndex).free->text().trimmed();
            if (!text.isEmpty()) select(blockIndex, text);
        });
        options->addWidget(block.free);
        body->addLayout(options);
        m_blocks << block;
    }
    m_instant = m_blocks.size() == 1 && !m_blocks.constFirst().multi;
    auto *actions = new QHBoxLayout;
    m_submit = new QPushButton(QStringLiteral("Responder"), m_body);
    m_submit->setObjectName(QStringLiteral("permissionPrimary"));
    m_submit->setCursor(Qt::PointingHandCursor);
    m_submit->setVisible(!m_instant);
    connect(m_submit, &QPushButton::clicked, this, &ChatQuestionCard::submit);
    auto *skip = new QPushButton(QStringLiteral("Ahora no"), m_body);
    skip->setObjectName(QStringLiteral("permissionSkip"));
    skip->setCursor(Qt::PointingHandCursor);
    connect(skip, &QPushButton::clicked, this, [this] {
        if (m_decided) return;
        emit answered({});
    });
    actions->addWidget(m_submit);
    actions->addStretch(1);
    actions->addWidget(skip);
    body->addLayout(actions);
    column->addWidget(m_body);
    m_decision = label(QString(), QStringLiteral("permissionDecision"), frame);
    column->addWidget(m_decision);
    outer->addWidget(frame);
    refresh(entry);
    update();
}

void ChatQuestionCard::select(int question, const QString &answer)
{
    if (m_decided || question < 0 || question >= m_blocks.size()) return;
    Block &block = m_blocks[question];
    if (block.multi) {
        if (!block.chosen.removeOne(answer)) block.chosen << answer;
    } else {
        block.chosen = {answer};
    }
    update();
    if (m_instant) submit();
}

void ChatQuestionCard::update()
{
    for (const Block &block : std::as_const(m_blocks)) {
        for (QPushButton *button : block.buttons) button->setChecked(block.chosen.contains(button->property("answer").toString()));
    }
    bool complete = true;
    for (const Block &block : std::as_const(m_blocks)) complete = complete && !block.chosen.isEmpty();
    m_submit->setEnabled(complete);
}

void ChatQuestionCard::submit()
{
    if (m_decided) return;
    QJsonObject answers;
    for (const Block &block : std::as_const(m_blocks)) {
        if (block.chosen.isEmpty()) return;
        answers.insert(block.question, block.chosen.join(QStringLiteral(", ")));
    }
    emit answered(answers);
}

void ChatQuestionCard::refresh(const AgentChatModel::Entry &entry)
{
    m_decided = !entry.decision.isEmpty();
    m_body->setVisible(!m_decided);
    m_decision->setVisible(m_decided);
    m_decision->setText(entry.decision == QStringLiteral("Ahora no") ? QStringLiteral("No respondiste.")
                                                                      : QStringLiteral("✓  %1").arg(entry.decision));
}
