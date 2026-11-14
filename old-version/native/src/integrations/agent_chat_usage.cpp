#include "agent_chat_usage.hpp"

#include <QDateTime>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QProgressBar>
#include <QStyle>

namespace {

QString level(double percent)
{
    if (percent >= 85) return QStringLiteral("high");
    if (percent >= 60) return QStringLiteral("medium");
    return QStringLiteral("low");
}

QString resetText(qint64 epoch)
{
    if (epoch <= 0) return {};
    const QDateTime reset = QDateTime::fromSecsSinceEpoch(epoch);
    const QString format = reset.date() == QDate::currentDate() ? QStringLiteral("HH:mm")
                                                                : QStringLiteral("ddd d, HH:mm");
    return QStringLiteral("Se reinicia: %1").arg(QLocale().toString(reset, format));
}

} // namespace

ChatUsageBar::ChatUsageBar(QWidget *parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("usageBar"));
    auto *row = new QHBoxLayout(this);
    row->setContentsMargins(0, 0, 0, 0);
    row->setSpacing(8);
    m_context = addMeter(QStringLiteral("Contexto"));
    m_primary = addMeter(QStringLiteral("5 h"));
    m_secondary = addMeter(QStringLiteral("Semana"));
    m_spend = new QLabel(this);
    m_spend->setObjectName(QStringLiteral("usageSpend"));
    m_spend->setToolTip(QStringLiteral("Tokens y costo estimado de esta conversación."));
    m_spend->hide();
    row->addWidget(m_spend);
    row->addStretch(1);
    setVisible(false);
}

ChatUsageBar::Meter ChatUsageBar::addMeter(const QString &label)
{
    Meter meter;
    meter.pill = new QWidget(this);
    meter.pill->setObjectName(QStringLiteral("usagePill"));
    auto *row = new QHBoxLayout(meter.pill);
    row->setContentsMargins(9, 3, 9, 3);
    row->setSpacing(6);
    meter.label = new QLabel(label, meter.pill);
    meter.label->setObjectName(QStringLiteral("usageLabel"));
    meter.bar = new QProgressBar(meter.pill);
    meter.bar->setObjectName(QStringLiteral("usageMeter"));
    meter.bar->setRange(0, 100);
    meter.bar->setTextVisible(false);
    meter.bar->setFixedSize(54, 5);
    meter.value = new QLabel(meter.pill);
    meter.value->setObjectName(QStringLiteral("usageValue"));
    row->addWidget(meter.label);
    row->addWidget(meter.bar);
    row->addWidget(meter.value);
    meter.pill->hide();
    layout()->addWidget(meter.pill);
    return meter;
}

void ChatUsageBar::showMeter(const Meter &meter, double percent, const QString &value, const QString &tooltip)
{
    meter.bar->setValue(qBound(0, qRound(percent), 100));
    meter.bar->setProperty("level", level(percent));
    meter.bar->style()->unpolish(meter.bar);
    meter.bar->style()->polish(meter.bar);
    meter.value->setText(value);
    meter.pill->setToolTip(tooltip);
    meter.pill->show();
}

void ChatUsageBar::setUsage(const AgentChatModel::Usage &usage, const QString &spend)
{
    if (usage.contextWindow > 0) {
        const double percent = usage.contextTokens * 100.0 / usage.contextWindow;
        showMeter(m_context, percent, QStringLiteral("%1%").arg(percent, 0, 'f', 0),
            QStringLiteral("%1 de %2 tokens de la ventana del modelo.")
                .arg(compactTokens(usage.contextTokens), compactTokens(usage.contextWindow)));
    } else {
        m_context.pill->hide();
    }
    const auto window = [](const Meter &meter, const AgentChatModel::LimitWindow &limit, const QString &name) {
        if (limit.usedPercent < 0) {
            meter.pill->hide();
            return;
        }
        showMeter(meter, limit.usedPercent, QStringLiteral("%1%").arg(limit.usedPercent, 0, 'f', 0),
            QStringLiteral("%1: %2% usado, %3% disponible.\n%4").arg(name).arg(limit.usedPercent, 0, 'f', 0)
                .arg(100 - limit.usedPercent, 0, 'f', 0).arg(resetText(limit.resetsAt)));
    };
    window(m_primary, usage.primary, usage.primary.windowMinutes == 300 ? QStringLiteral("Límite de 5 horas")
                                                                        : QStringLiteral("Límite corto"));
    window(m_secondary, usage.secondary, QStringLiteral("Límite semanal"));
    m_spend->setText(spend);
    m_spend->setVisible(!spend.isEmpty());
    if (!usage.plan.isEmpty()) m_spend->setToolTip(QStringLiteral("Plan %1. Tokens y costo de esta conversación.").arg(usage.plan));
    setVisible(m_context.pill->isVisibleTo(this) || m_primary.pill->isVisibleTo(this)
        || m_secondary.pill->isVisibleTo(this) || !spend.isEmpty());
}

QString ChatUsageBar::summary() const
{
    QStringList parts;
    for (const Meter *meter : {&m_context, &m_primary, &m_secondary}) {
        if (meter->pill->isVisibleTo(this)) parts << meter->label->text() + QLatin1Char(' ') + meter->value->text();
    }
    if (m_spend->isVisibleTo(this)) parts << m_spend->text();
    return parts.join(QStringLiteral(" · "));
}
