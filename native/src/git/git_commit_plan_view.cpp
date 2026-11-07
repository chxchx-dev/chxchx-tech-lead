#include "git_commit_plan_view.hpp"

#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>

GitCommitPlanView::GitCommitPlanView(QWidget *parent) : QFrame(parent)
{
    setObjectName(QStringLiteral("gitCommitPlan"));
    setFrameShape(QFrame::StyledPanel);
    auto *column = new QVBoxLayout(this);
    column->setContentsMargins(10, 8, 10, 8);
    column->setSpacing(6);
    auto *header = new QHBoxLayout;
    m_title = new QLabel(this);
    m_title->setWordWrap(true);
    auto *close = new QPushButton(QStringLiteral("Cerrar plan"), this);
    close->setFlat(true);
    close->setCursor(Qt::PointingHandCursor);
    connect(close, &QPushButton::clicked, this, &GitCommitPlanView::dismissed);
    header->addWidget(m_title, 1);
    header->addWidget(close);
    column->addLayout(header);
    m_rows = new QVBoxLayout;
    m_rows->setSpacing(4);
    column->addLayout(m_rows);
    hide();
}

void GitCommitPlanView::setPlan(const QList<GitCommitGroup> &plan, const QString &agentLabel)
{
    m_plan = plan;
    if (!agentLabel.isEmpty()) m_agent = agentLabel;
    rebuild();
}

void GitCommitPlanView::rebuild()
{
    while (QLayoutItem *item = m_rows->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    if (m_plan.isEmpty()) return hide();
    m_title->setText(QStringLiteral("<b>%1 commits agrupados por tema</b> · %2<br>Prepara uno, revísalo y guárdalo; el siguiente queda a la espera.")
                         .arg(m_plan.size()).arg(m_agent.toHtmlEscaped()));
    for (int index = 0; index < m_plan.size(); ++index) {
        const GitCommitGroup &group = m_plan.at(index);
        auto *row = new QWidget(this);
        auto *line = new QHBoxLayout(row);
        line->setContentsMargins(0, 0, 0, 0);
        auto *text = new QLabel(QStringLiteral("%1. %2<br><small>%3</small>").arg(index + 1)
            .arg(group.message.section(QLatin1Char('\n'), 0, 0).toHtmlEscaped())
            .arg(QStringLiteral("%1 archivo(s): %2").arg(group.files.size()).arg(group.files.join(QStringLiteral(", ")).toHtmlEscaped())), row);
        text->setWordWrap(true);
        text->setTextFormat(Qt::RichText);
        auto *prepare = new QPushButton(index == 0 ? QStringLiteral("Preparar") : QStringLiteral("Preparar este"), row);
        prepare->setCursor(Qt::PointingHandCursor);
        connect(prepare, &QPushButton::clicked, this, [this, index] { emit prepareRequested(index); });
        line->addWidget(text, 1);
        line->addWidget(prepare, 0, Qt::AlignTop);
        m_rows->addWidget(row);
    }
    show();
}
