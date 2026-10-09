#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/agent_session_widget.hpp"
#include "studio_icons.hpp"

#include <QComboBox>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QListWidgetItem>
#include <QStyle>

namespace {

QString areaLabel(const QString &id)
{
    for (const auto &area : areas) {
        if (id == QString::fromUtf8(area.id)) return QString::fromUtf8(area.label);
    }
    return id;
}

QString joinedValues(const QJsonArray &values)
{
    QStringList result;
    for (const auto &value : values) result << value.toString();
    return result.isEmpty() ? QStringLiteral("sin detectar") : result.join(QStringLiteral(" · "));
}

} // namespace

void MainWindow::showAreaDashboard(const QJsonObject &payload, const QString &area)
{
    if (area != m_currentArea || !m_dashboardItems) return;
    const QJsonObject project = payload.value(QStringLiteral("project")).toObject();
    const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
    m_homeQuickActions->setVisible(area == QStringLiteral("overview"));
    if (area == QStringLiteral("overview")) {
        m_homeAgentSelector->clear();
        for (const auto &value : payload.value(QStringLiteral("agents")).toArray()) {
            const QJsonObject agent = value.toObject();
            const QString id = agent.value(QStringLiteral("id")).toString();
            if (id.isEmpty()) continue;
            m_homeAgentSelector->addItem(id, agent.value(QStringLiteral("status")).toString());
            const int index = m_homeAgentSelector->count() - 1;
            m_homeAgentSelector->setItemData(index, agent.value(QStringLiteral("command")).toString(), Qt::UserRole + 1);
            m_homeAgentSelector->setItemData(index, agent.value(QStringLiteral("cwd")).toString(), Qt::UserRole + 2);
            m_homeAgentSelector->setItemData(index, QString::fromUtf8(
                QJsonDocument(agent.value(QStringLiteral("arguments")).toArray())
                    .toJson(QJsonDocument::Compact)), Qt::UserRole + 3);
            m_homeAgentSelector->setItemData(index, agent.value(QStringLiteral("shell")).toBool(), Qt::UserRole + 4);
            m_homeAgentSelector->setItemData(index, QString::fromUtf8(
                QJsonDocument(agent.value(QStringLiteral("studio_command")).toArray())
                    .toJson(QJsonDocument::Compact)), Qt::UserRole + 5);
            m_homeAgentSelector->setItemData(index, QString::fromUtf8(
                QJsonDocument(agent.value(QStringLiteral("studio_new_chat_command")).toArray())
                    .toJson(QJsonDocument::Compact)), Qt::UserRole + 6);
        }
    }
    updateHomeActionState();
    const auto addRow = [this](const QString &text, const QString &iconName,
        QStyle::StandardPixmap fallback) {
        auto *item = new QListWidgetItem(studioIcon(iconName, fallback), text, m_dashboardItems);
        item->setToolTip(text);
        item->setSizeHint(QSize(0, 52));
        return item;
    };

    m_dashboardTitle->setText(areaLabel(area));
    m_dashboardItems->clear();

    if (area == QStringLiteral("resources")) {
        const QJsonObject system = payload.value(QStringLiteral("system")).toObject();
        const QJsonArray projects = payload.value(QStringLiteral("projects")).toArray();
        m_dashboardSummary->setText(QStringLiteral("Sistema · RAM %1% · swap %2% · CPU %3% · %4 proyectos registrados")
            .arg(QString::number(system.value(QStringLiteral("memory_percent")).toDouble(), 'f', 0),
                 QString::number(system.value(QStringLiteral("swap_percent")).toDouble(), 'f', 0),
                 QString::number(system.value(QStringLiteral("cpu_percent")).toDouble(), 'f', 0),
                 QString::number(projects.size())));
        for (const auto &value : projects) {
            const auto entry = value.toObject();
            addRow(QStringLiteral("%1 · %2\n%3 · %4 proceso(s) · RAM %5 · CPU %6")
                    .arg(entry.value(QStringLiteral("alias")).toString(),
                         entry.value(QStringLiteral("name")).toString(),
                         entry.value(QStringLiteral("status")).toString(),
                         QString::number(entry.value(QStringLiteral("running_count")).toInt()),
                         QString::number(entry.value(QStringLiteral("rss_bytes")).toDouble() / (1024.0 * 1024.0), 'f', 0),
                         QString::number(entry.value(QStringLiteral("cpu_percent")).toDouble(), 'f', 0)),
                QStringLiteral("drive-harddisk"), QStyle::SP_DriveHDIcon);
        }
        if (projects.isEmpty()) addRow(QStringLiteral("No hay proyectos registrados."),
            QStringLiteral("dialog-information"), QStyle::SP_MessageBoxInformation);
        return;
    }

    if (area == QStringLiteral("skills")) {
        const QJsonArray skills = payload.value(QStringLiteral("skills")).toArray();
        const int enabled = payload.value(QStringLiteral("enabled_skill_count")).toInt();
        int recommended = 0;
        for (const auto &value : skills) {
            const auto skill = value.toObject();
            const bool isEnabled = skill.value(QStringLiteral("enabled")).toBool();
            const QJsonArray reasons = skill.value(QStringLiteral("reasons")).toArray();
            const bool isRecommended = skill.value(QStringLiteral("recommended")).toBool() || !reasons.isEmpty();
            if (isRecommended) ++recommended;
            const QString state = isEnabled ? QStringLiteral("HABILITADA") : QStringLiteral("No habilitada");
            const QString hint = isRecommended ? QStringLiteral(" · RECOMENDADA") : QString();
            addRow(QStringLiteral("%1 · %2%3\n%4")
                    .arg(skill.value(QStringLiteral("name")).toString(), state, hint,
                         skill.value(QStringLiteral("description")).toString()),
                isEnabled ? QStringLiteral("emblem-ok") : QStringLiteral("preferences-plugin"),
                isEnabled ? QStyle::SP_DialogApplyButton : QStyle::SP_FileIcon);
        }
        m_dashboardSummary->setText(QStringLiteral("%1 de %2 skills habilitadas · %3 recomendadas para este proyecto")
            .arg(QString::number(enabled), QString::number(skills.size()), QString::number(recommended)));
        if (skills.isEmpty()) addRow(QStringLiteral("El catálogo de skills está vacío."),
            QStringLiteral("dialog-information"), QStyle::SP_MessageBoxInformation);
        return;
    }

    if (area == QStringLiteral("agents")) {
        const QJsonArray agents = payload.value(QStringLiteral("agents")).toArray();
        int available = 0;
        for (const auto &value : agents) {
            const auto agent = value.toObject();
            const QString id = agent.value(QStringLiteral("id")).toString();
            const bool isAvailable = agent.value(QStringLiteral("available")).toBool();
            if (isAvailable) ++available;
            const QString detail = QStringLiteral("Sesión: %1 · pane: %2")
                .arg(agent.value(QStringLiteral("session")).toString(QStringLiteral("sin sesión")),
                     agent.value(QStringLiteral("pane")).toString(QStringLiteral("sin pane")));
            auto *row = addRow(QStringLiteral("%1 · %2\n%3")
                    .arg(id, isAvailable ? QStringLiteral("Disponible") : QStringLiteral("No instalado"), detail),
                isAvailable ? QStringLiteral("system-run") : QStringLiteral("dialog-warning"),
                isAvailable ? QStyle::SP_CommandLink : QStyle::SP_MessageBoxWarning);
            row->setData(Qt::UserRole, QStringLiteral("agent"));
            row->setData(Qt::UserRole + 1, id);
            row->setData(Qt::UserRole + 2, isAvailable);
            row->setData(Qt::UserRole + 3, detail);
        }
        m_dashboardSummary->setText(QStringLiteral("%1 agentes configurados · %2 disponibles · %3 activos en Studio")
            .arg(QString::number(agents.size()), QString::number(available),
                 QString::number(currentProjectEmbeddedAgentCount())));
        if (agents.isEmpty()) addRow(QStringLiteral("No hay agentes configurados para este proyecto."),
            QStringLiteral("dialog-information"), QStyle::SP_MessageBoxInformation);
        updateAgentDashboard();
        return;
    }

    if (area == QStringLiteral("processes")) {
        const QJsonArray processes = payload.value(QStringLiteral("processes")).toArray();
        int running = 0;
        for (const auto &value : processes) {
            const auto process = value.toObject();
            const QString state = process.value(QStringLiteral("status")).toString();
            const bool active = state == QStringLiteral("RUNNING");
            if (active) ++running;
            QString detail = process.value(QStringLiteral("label")).toString();
            if (process.value(QStringLiteral("pid")).isDouble())
                detail += QStringLiteral(" · PID %1").arg(process.value(QStringLiteral("pid")).toInt());
            addRow(QStringLiteral("%1 · %2").arg(detail, active ? QStringLiteral("En ejecución") : state),
                active ? QStringLiteral("media-playback-start") : QStringLiteral("application-x-executable"),
                active ? QStyle::SP_MediaPlay : QStyle::SP_ComputerIcon);
        }
        m_dashboardSummary->setText(QStringLiteral("%1 de %2 procesos en ejecución")
            .arg(QString::number(running), QString::number(processes.size())));
        if (processes.isEmpty()) addRow(QStringLiteral("No hay procesos configurados."),
            QStringLiteral("dialog-information"), QStyle::SP_MessageBoxInformation);
        return;
    }

    if (area == QStringLiteral("packs")) {
        const QJsonArray packs = payload.value(QStringLiteral("packs")).toArray();
        int detected = 0;
        for (const auto &value : packs) {
            const auto pack = value.toObject();
            const bool matches = !pack.value(QStringLiteral("reasons")).toArray().isEmpty();
            if (matches) ++detected;
            addRow(QStringLiteral("%1 · %2\n%3 skill(s) · %4")
                    .arg(pack.value(QStringLiteral("name")).toString(),
                         matches ? QStringLiteral("Detectado") : QStringLiteral("Disponible"),
                         QString::number(pack.value(QStringLiteral("skills")).toArray().size()),
                         pack.value(QStringLiteral("description")).toString()),
                matches ? QStringLiteral("package-x-generic") : QStringLiteral("package-x-generic"),
                matches ? QStyle::SP_DirLinkIcon : QStyle::SP_FileIcon);
        }
        m_dashboardSummary->setText(QStringLiteral("%1 Tech Packs disponibles · %2 detectados")
            .arg(QString::number(packs.size()), QString::number(detected)));
        return;
    }

    if (area == QStringLiteral("projects")) {
        const QJsonArray projects = payload.value(QStringLiteral("registered_projects")).toArray();
        int trusted = 0;
        for (const auto &value : projects) {
            const auto entry = value.toObject();
            const bool isTrusted = entry.value(QStringLiteral("trusted")).toBool();
            if (isTrusted) ++trusted;
            addRow(QStringLiteral("%1 · %2\n%3 · %4")
                    .arg(entry.value(QStringLiteral("alias")).toString(),
                         entry.value(QStringLiteral("name")).toString(),
                         entry.value(QStringLiteral("status")).toString(),
                         isTrusted ? QStringLiteral("Confiable") : QStringLiteral("Requiere confianza")),
                isTrusted ? QStringLiteral("folder-open") : QStringLiteral("dialog-warning"),
                isTrusted ? QStyle::SP_DirOpenIcon : QStyle::SP_MessageBoxWarning);
        }
        m_dashboardSummary->setText(QStringLiteral("%1 proyectos registrados · %2 confiables")
            .arg(QString::number(projects.size()), QString::number(trusted)));
        return;
    }

    const QString trust = workspace.value(QStringLiteral("trusted")).toBool()
        ? QStringLiteral("Confiable") : QStringLiteral("Requiere confianza");
    const QString status = workspace.value(QStringLiteral("status")).toString(QStringLiteral("Sin iniciar"));
    const QString stack = joinedValues(project.value(QStringLiteral("stacks")).toArray());
    m_dashboardSummary->setText(QStringLiteral("%1 · workspace %2 · %3")
        .arg(project.value(QStringLiteral("name")).toString(), status, trust));
    addRow(QStringLiteral("Stack detectado\n%1").arg(stack),
        QStringLiteral("applications-development"), QStyle::SP_ComputerIcon);
    addRow(QStringLiteral("Sesión\n%1").arg(workspace.value(QStringLiteral("session_name")).toString(
            QStringLiteral("No hay sesión activa"))),
        QStringLiteral("utilities-terminal"), QStyle::SP_ComputerIcon);
    const QJsonObject resources = payload.value(QStringLiteral("resources")).toObject();
    const QJsonObject memory = resources.value(QStringLiteral("memory")).toObject();
    const QJsonObject swap = resources.value(QStringLiteral("swap")).toObject();
    addRow(QStringLiteral("Recursos\nRAM %1% · swap %2% · CPU %3%")
            .arg(QString::number(memory.value(QStringLiteral("percent")).toDouble(), 'f', 0),
                 QString::number(swap.value(QStringLiteral("percent")).toDouble(), 'f', 0),
                 QString::number(resources.value(QStringLiteral("cpu_percent")).toDouble(), 'f', 0)),
        QStringLiteral("drive-harddisk"), QStyle::SP_DriveHDIcon);
}

void MainWindow::updateAgentDashboard()
{
    int configured = 0;
    int available = 0;
    int active = 0;
    const QString projectKey = QDir::cleanPath(QFileInfo(m_projectPath).absoluteFilePath());
    for (int index = 0; index < m_dashboardItems->count(); ++index) {
        auto *item = m_dashboardItems->item(index);
        if (item->data(Qt::UserRole).toString() != QStringLiteral("agent")) continue;
        ++configured;
        const QString id = item->data(Qt::UserRole + 1).toString();
        const bool isAvailable = item->data(Qt::UserRole + 2).toBool();
        if (isAvailable) ++available;
        const QString keyPrefix = projectKey + QLatin1Char('#') + id;
        bool isActive = false;
        for (auto session = m_agentSessionProjects.cbegin(); session != m_agentSessionProjects.cend(); ++session) {
            if (QDir::cleanPath(QFileInfo(session.value()).absoluteFilePath()) == projectKey
                && (session.key() == keyPrefix || session.key().startsWith(keyPrefix + QLatin1Char('#')))
                && m_agentSessions.value(session.key()) != nullptr
                && m_agentSessions.value(session.key())->isRunning()) {
                isActive = true;
                break;
            }
        }
        if (isActive) ++active;
        const QString state = isActive ? QStringLiteral("En uso en Studio")
            : (isAvailable ? QStringLiteral("Disponible") : QStringLiteral("No instalado"));
        item->setText(QStringLiteral("%1 · %2\n%3")
            .arg(id, state, item->data(Qt::UserRole + 3).toString()));
        item->setIcon(studioIcon(isActive ? QStringLiteral("media-playback-start")
                                         : (isAvailable ? QStringLiteral("system-run") : QStringLiteral("dialog-warning")),
            isActive ? QStyle::SP_MediaPlay : (isAvailable ? QStyle::SP_CommandLink : QStyle::SP_MessageBoxWarning)));
    }
    if (m_currentArea == QStringLiteral("agents")) {
        m_dashboardSummary->setText(QStringLiteral("%1 agentes configurados · %2 disponibles · %3 activos en Studio")
            .arg(QString::number(configured), QString::number(available), QString::number(active)));
    }
}
