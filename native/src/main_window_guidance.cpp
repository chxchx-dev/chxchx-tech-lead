#include "main_window.hpp"

#include <QComboBox>
#include <QFileInfo>
#include <QGroupBox>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>

// Guía de Inicio: calcula, a partir del estado real del proyecto, qué falta y cuál es el
// siguiente paso. Solo recomienda; cada acción reutiliza el flujo con vista previa existente.

void MainWindow::buildHomeGuidance(QWidget *parent, QVBoxLayout *layout)
{
    m_homeGuidance = new QGroupBox(QStringLiteral("Siguiente paso recomendado"), parent);
    m_homeGuidance->setObjectName(QStringLiteral("homeGuidance"));
    auto *box = new QVBoxLayout(m_homeGuidance);
    m_homeGuidanceSteps = new QLabel(m_homeGuidance);
    m_homeGuidanceSteps->setWordWrap(true);
    m_homeGuidanceSteps->setTextFormat(Qt::RichText);
    m_homeGuidanceButton = new QPushButton(m_homeGuidance);
    box->addWidget(m_homeGuidanceSteps);
    box->addWidget(m_homeGuidanceButton, 0, Qt::AlignLeft);
    layout->addWidget(m_homeGuidance);
    connect(m_homeGuidanceButton, &QPushButton::clicked, this, &MainWindow::runHomeGuidanceAction);
}

void MainWindow::updateHomeGuidance(const QJsonObject &payload)
{
    if (m_homeGuidance == nullptr) return;
    struct Step { QString title; QString hint; bool done; QString action; QString button; };
    const bool prepared = QFileInfo(m_projectPath + QStringLiteral("/.ai/chxchx-tech.toml")).isFile();
    bool hasAgent = false;
    for (int i = 0; i < m_homeAgentSelector->count(); ++i) {
        if (!m_homeAgentSelector->itemData(i, Qt::UserRole + 1).toString().isEmpty()
            && !m_homeAgentSelector->itemData(i, Qt::UserRole + 4).toBool()) hasAgent = true;
    }
    bool skillsEnabled = false;
    for (const auto &value : payload.value(QStringLiteral("skills")).toArray())
        skillsEnabled = skillsEnabled || value.toObject().value(QStringLiteral("enabled")).toBool();

    const QList<Step> steps = {
        {QStringLiteral("Preparar el proyecto"),
         QStringLiteral("Crea la configuración de ChxChx (.ai) y aplica las skills del stack. Verás una vista previa antes."),
         prepared, QStringLiteral("prepare"), QStringLiteral("Preparar y entrar")},
        {QStringLiteral("Confiar en el proyecto"),
         QStringLiteral("Autoriza que ChxChx ejecute los procesos y agentes configurados aquí. Es una confirmación tuya."),
         prepared && m_projectTrusted, QStringLiteral("trust"), QStringLiteral("Confiar proyecto")},
        {QStringLiteral("Tener un agente"),
         QStringLiteral("Añade OpenCode, o instala Codex/Claude y vuelve a Inicio."),
         hasAgent, QStringLiteral("add-agent"), QStringLiteral("Añadir OpenCode")},
        {QStringLiteral("Elegir skills (opcional)"),
         QStringLiteral("Habilita las recomendadas para el stack para que el agente trabaje con tus reglas."),
         skillsEnabled, QStringLiteral("skills"), QStringLiteral("Ver skills")},
        {QStringLiteral("Abrir un agente"),
         QStringLiteral("Elige el agente en Acciones rápidas y pulsa Nueva sesión o Chat visual."),
         false, QStringLiteral("agent"), QStringLiteral("Abrir agente")},
    };
    int current = -1;
    for (int i = 0; i < steps.size(); ++i) {
        if (!steps.at(i).done) { current = i; break; }
    }
    QStringList lines;
    for (int i = 0; i < steps.size(); ++i) {
        const Step &step = steps.at(i);
        const QString mark = step.done ? QStringLiteral("✓") : (i == current ? QStringLiteral("→") : QStringLiteral("○"));
        const QString text = i == current
            ? QStringLiteral("<b>%1 %2. %3</b><br>&nbsp;&nbsp;&nbsp;&nbsp;%4")
                  .arg(mark, QString::number(i + 1), step.title.toHtmlEscaped(), step.hint.toHtmlEscaped())
            : QStringLiteral("%1 %2. %3").arg(mark, QString::number(i + 1), step.title.toHtmlEscaped());
        lines << text;
    }
    m_homeGuidanceSteps->setText(lines.join(QStringLiteral("<br>")));
    m_homeGuidanceAction = steps.at(current).action;
    m_homeGuidanceButton->setText(steps.at(current).button);
    m_homeGuidanceButton->setEnabled(true);
}

void MainWindow::runHomeGuidanceAction()
{
    if (m_homeGuidanceAction == QStringLiteral("prepare")) prepareProjectFromHome();
    else if (m_homeGuidanceAction == QStringLiteral("trust")) trustProjectFromHome();
    else if (m_homeGuidanceAction == QStringLiteral("add-agent")) addAgentToProject(QStringLiteral("opencode"));
    else if (m_homeGuidanceAction == QStringLiteral("skills")) showArea(QStringLiteral("skills"));
    else if (m_homeGuidanceAction == QStringLiteral("agent")) openHomeSelectedAgent(true);
}
