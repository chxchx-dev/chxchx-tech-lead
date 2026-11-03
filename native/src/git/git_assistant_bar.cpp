#include "git_assistant_bar.hpp"

#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QHBoxLayout>
#include <QLabel>
#include <QMenu>
#include <QPushButton>
#include <QToolButton>
#include <QVBoxLayout>

GitAssistantBar::GitAssistantBar(QWidget *parent) : QFrame(parent)
{
    setObjectName(QStringLiteral("gitAssistantBar"));
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(12, 10, 10, 10);
    layout->setSpacing(6);

    auto *top = new QHBoxLayout;
    top->setSpacing(8);
    m_icon = new QLabel(this);
    m_icon->setFixedSize(22, 22);
    m_title = new QLabel(this);
    m_title->setWordWrap(true);
    QFont bold = m_title->font();
    bold.setBold(true);
    m_title->setFont(bold);
    m_agent = new QToolButton(this);
    m_agent->setObjectName(QStringLiteral("gitAgentButton"));
    m_agent->setPopupMode(QToolButton::InstantPopup);
    m_agent->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_agent->setIcon(studioIcon(QStringLiteral("studio-assistant"), QStyle::SP_MessageBoxInformation));
    m_agent->setAutoRaise(true);
    m_agentMenu = new QMenu(m_agent);
    m_agent->setMenu(m_agentMenu);
    top->addWidget(m_icon, 0, Qt::AlignTop);
    top->addWidget(m_title, 1);
    top->addWidget(m_agent, 0, Qt::AlignTop);
    layout->addLayout(top);

    m_steps = new QLabel(this);
    m_steps->setWordWrap(true);
    m_steps->setTextFormat(Qt::RichText);
    layout->addWidget(m_steps);

    auto *bottom = new QHBoxLayout;
    m_advice = new QPushButton(studioIcon(QStringLiteral("studio-advice"), QStyle::SP_MessageBoxQuestion), QStringLiteral("Pedir consejo"), this);
    m_advice->setToolTip(QStringLiteral("El asistente explica qué está pasando y qué hacer, paso a paso. Solo aconseja: no ejecuta nada."));
    m_busyLabel = new QLabel(this);
    m_busyLabel->setWordWrap(true);
    m_cancel = new QPushButton(studioIcon(QStringLiteral("close-small"), QStyle::SP_DialogCancelButton), QStringLiteral("Cancelar"), this);
    m_hint = new QLabel(this);
    m_hint->setWordWrap(true);
    bottom->addWidget(m_advice);
    bottom->addWidget(m_busyLabel, 1);
    bottom->addWidget(m_cancel);
    bottom->addStretch(1);
    layout->addLayout(bottom);
    layout->addWidget(m_hint);
    m_busyLabel->hide();
    m_cancel->hide();

    connect(m_advice, &QPushButton::clicked, this, [this] { requestAdvice(); });
    connect(m_cancel, &QPushButton::clicked, this, &GitAssistantBar::cancelRequested);
    connect(m_agentMenu, &QMenu::triggered, this, [this](QAction *action) { emit agentChosen(action->data().toString()); });
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, [this] { refreshColors(); });
    setAgents({}, QString());
    applyTone();
}

void GitAssistantBar::setGuidance(const GitGuidance &guidance)
{
    m_guidance = guidance;
    m_title->setText(guidance.title);
    QStringList items;
    for (int i = 0; i < guidance.steps.size(); ++i) items << QStringLiteral("<li>%1</li>").arg(guidance.steps.at(i).toHtmlEscaped());
    m_steps->setText(guidance.steps.size() == 1
        ? guidance.steps.first().toHtmlEscaped()
        : QStringLiteral("<ol style='margin-top:0; margin-bottom:0; margin-left:-18px'>%1</ol>").arg(items.join(QString())));
    m_steps->setVisible(!guidance.steps.isEmpty() && !(guidance.tone == GitGuidance::Tone::Done && guidance.steps.size() == 1));
    // Nothing to ask about when everything is in order.
    m_advice->setVisible(guidance.tone != GitGuidance::Tone::Done || guidance.steps.size() > 1);
    applyTone();
}

void GitAssistantBar::setAgents(const QList<GitAssistantAgent> &agents, const QString &currentId)
{
    m_hasAgents = !agents.isEmpty();
    m_agentMenu->clear();
    QString label = QStringLiteral("Sin agente");
    for (const GitAssistantAgent &agent : agents) {
        QAction *action = m_agentMenu->addAction(agent.label);
        action->setData(agent.id);
        action->setCheckable(true);
        action->setChecked(agent.id == currentId);
        if (agent.id == currentId) label = agent.label;
    }
    m_agent->setText(label);
    m_agent->setEnabled(m_hasAgents && !m_busy);
    m_agent->setToolTip(m_hasAgents ? QStringLiteral("Agente que responde. Elige otro si lo prefieres.")
                                    : QStringLiteral("Aún no hay un agente conectado."));
    m_advice->setEnabled(m_hasAgents && !m_busy);
    m_hint->setVisible(!m_hasAgents);
    m_hint->setText(QStringLiteral("Conecta un agente (Codex, Claude Code u OpenCode) en Agentes para pedir consejo y sugerencias de mensajes."));
    QPalette palette = m_hint->palette();
    palette.setColor(QPalette::WindowText, ThemeManager::current().textMuted);
    m_hint->setPalette(palette);
}

void GitAssistantBar::setBusy(bool busy, const QString &text)
{
    m_busy = busy;
    m_busyLabel->setVisible(busy);
    m_busyLabel->setText(text);
    m_cancel->setVisible(busy);
    m_advice->setVisible(!busy && (m_guidance.tone != GitGuidance::Tone::Done || m_guidance.steps.size() > 1));
    m_advice->setEnabled(m_hasAgents && !busy);
    m_agent->setEnabled(m_hasAgents && !busy);
}

void GitAssistantBar::refreshColors()
{
    applyTone();
    QPalette palette = m_hint->palette();
    palette.setColor(QPalette::WindowText, ThemeManager::current().textMuted);
    m_hint->setPalette(palette);
}

void GitAssistantBar::applyTone()
{
    const StudioTheme &theme = ThemeManager::current();
    QColor color = theme.accent;
    if (m_guidance.tone == GitGuidance::Tone::Warning) color = theme.warning;
    else if (m_guidance.tone == GitGuidance::Tone::Done) color = theme.success;
    QColor soft = color;
    soft.setAlpha(28);
    setStyleSheet(QStringLiteral("#gitAssistantBar { background-color: rgba(%1,%2,%3,%4); border: 1px solid rgba(%1,%2,%3,90); border-left: 3px solid %5; border-radius: 8px; }"
                                 "#gitAssistantBar QLabel { background: transparent; }")
        .arg(soft.red()).arg(soft.green()).arg(soft.blue()).arg(soft.alpha()).arg(color.name()));
    m_icon->setPixmap(renderStudioIcon(m_guidance.icon.isEmpty() ? QStringLiteral("studio-git") : m_guidance.icon, color, 20));
    QPalette palette = m_steps->palette();
    palette.setColor(QPalette::WindowText, theme.textMuted);
    m_steps->setPalette(palette);
}

QString GitAssistantBar::titleText() const { return m_title->text(); }
QString GitAssistantBar::stepsText() const { return m_steps->text(); }
QString GitAssistantBar::agentText() const { return m_agent->text(); }
QString GitAssistantBar::hintText() const { return m_hint->isVisibleTo(this) ? m_hint->text() : QString(); }
bool GitAssistantBar::adviceEnabled() const { return m_advice->isEnabled() && m_advice->isVisibleTo(this); }
