#include "agent_chat_permission.hpp"

#include <QFontDatabase>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>

namespace {

constexpr int Indent = 38; // aligned with the agent's messages

QLabel *textLabel(const QString &objectName, QWidget *parent)
{
    auto *label = new QLabel(parent);
    label->setObjectName(objectName);
    label->setTextFormat(Qt::PlainText);
    label->setWordWrap(true);
    label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    return label;
}

} // namespace

ChatPermissionCard::ChatPermissionCard(const AgentChatModel::Entry &entry, QWidget *parent)
    : ChatEntryCard(parent)
{
    auto *outer = new QVBoxLayout(this);
    outer->setContentsMargins(Indent, 2, 0, 2);
    auto *frame = new QFrame(this);
    frame->setObjectName(QStringLiteral("permissionCard"));
    auto *column = new QVBoxLayout(frame);
    column->setContentsMargins(12, 10, 12, 10);
    column->setSpacing(6);
    m_title = textLabel(QStringLiteral("permissionTitle"), frame);
    m_summary = textLabel(QStringLiteral("permissionSummary"), frame);
    m_summary->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    m_detail = textLabel(QStringLiteral("permissionDetail"), frame);
    m_decision = textLabel(QStringLiteral("permissionDecision"), frame);
    m_buttons = new QWidget(frame);
    m_buttons->setObjectName(QStringLiteral("permissionButtons"));
    auto *row = new QHBoxLayout(m_buttons);
    row->setContentsMargins(0, 2, 0, 0);
    row->setSpacing(6);
    const QJsonArray choices = entry.choices;
    for (int index = 0; index < choices.size(); ++index) {
        auto *button = new QPushButton(choices.at(index).toObject().value(QStringLiteral("label")).toString(), m_buttons);
        button->setObjectName(index == 0 ? QStringLiteral("permissionPrimary") : QStringLiteral("permissionOption"));
        button->setCursor(Qt::PointingHandCursor);
        connect(button, &QPushButton::clicked, this, [this, index] { choose(index); });
        row->addWidget(button);
    }
    auto *skip = new QPushButton(QStringLiteral("Ahora no"), m_buttons);
    skip->setObjectName(QStringLiteral("permissionSkip"));
    skip->setCursor(Qt::PointingHandCursor);
    connect(skip, &QPushButton::clicked, this, [this] { choose(-1); });
    row->addStretch(1);
    row->addWidget(skip);
    column->addWidget(m_title);
    column->addWidget(m_summary);
    column->addWidget(m_detail);
    column->addWidget(m_buttons);
    column->addWidget(m_decision);
    outer->addWidget(frame);
    refresh(entry);
}

void ChatPermissionCard::refresh(const AgentChatModel::Entry &entry)
{
    m_title->setText(entry.certain
        ? QStringLiteral("🔐  Necesita tu permiso")
        : QStringLiteral("🔐  Puede que el sandbox lo haya bloqueado"));
    m_summary->setText(entry.text);
    m_detail->setText(entry.output);
    m_detail->setVisible(!entry.output.trimmed().isEmpty());
    m_decided = !entry.decision.isEmpty();
    m_buttons->setVisible(!m_decided);
    m_decision->setVisible(m_decided);
    m_decision->setText(entry.decision == QStringLiteral("Ahora no")
        ? QStringLiteral("Lo dejaste bloqueado.") : QStringLiteral("✓  %1").arg(entry.decision));
}

void ChatPermissionCard::choose(int choice)
{
    if (m_decided) return;
    emit choiceChosen(choice);
}
