#include "main_window.hpp"

#include <QJsonArray>
#include <QJsonObject>
#include <QStringList>

namespace {

QString formatBytes(double bytes)
{
    constexpr double kibibyte = 1024.0;
    constexpr double mebibyte = kibibyte * 1024.0;
    constexpr double gibibyte = mebibyte * 1024.0;
    if (bytes >= gibibyte) {
        return QStringLiteral("%1 GiB").arg(bytes / gibibyte, 0, 'f', 1);
    }
    if (bytes >= mebibyte) {
        return QStringLiteral("%1 MiB").arg(bytes / mebibyte, 0, 'f', 0);
    }
    return QStringLiteral("%1 KiB").arg(bytes / kibibyte, 0, 'f', 0);
}

QString displayPercent(const QJsonValue &value)
{
    return value.isDouble()
        ? QStringLiteral("%1%").arg(value.toDouble(), 0, 'f', 0)
        : QStringLiteral("no disponible");
}


} // namespace

QString MainWindow::formatBridgeStatus(const QJsonObject &payload, const QString &area) const
{
    const QJsonObject project = payload.value(QStringLiteral("project")).toObject();
    const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
    const QJsonObject resources = payload.value(QStringLiteral("resources")).toObject();
    const QJsonObject memory = resources.value(QStringLiteral("memory")).toObject();
    const QJsonObject swap = resources.value(QStringLiteral("swap")).toObject();
    QStringList lines;
    lines << QStringLiteral("%1 · %2")
        .arg(project.value(QStringLiteral("name")).toString(), project.value(QStringLiteral("profile")).toString());

    if (area == QStringLiteral("skills")) {
        const QJsonArray skills = payload.value(QStringLiteral("skills")).toArray();
        lines << QStringLiteral("Skills habilitadas: %1 · catálogo: %2")
            .arg(QString::number(payload.value(QStringLiteral("enabled_skill_count")).toInt()),
                 QString::number(skills.size()));
        for (const auto &value : skills) {
            const auto skill = value.toObject();
            QStringList details;
            if (skill.value(QStringLiteral("enabled")).toBool()) {
                details << QStringLiteral("habilitada");
            }
            for (const auto &reason : skill.value(QStringLiteral("reasons")).toArray()) {
                details << reason.toString();
            }
            lines << QStringLiteral("%1 — %2%3")
                .arg(skill.value(QStringLiteral("name")).toString(),
                     skill.value(QStringLiteral("description")).toString(),
                     details.isEmpty() ? QString() : QStringLiteral(" · ") + details.join(QStringLiteral("; ")));
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("packs")) {
        const QJsonArray packs = payload.value(QStringLiteral("packs")).toArray();
        lines << QStringLiteral("Tech Packs: %1").arg(packs.size());
        for (const auto &value : packs) {
            const auto pack = value.toObject();
            QStringList skillNames;
            for (const auto &skill : pack.value(QStringLiteral("skills")).toArray()) {
                skillNames << skill.toString();
            }
            QString detail = QStringLiteral("%1 — %2\n  Skills: %3")
                .arg(pack.value(QStringLiteral("name")).toString())
                .arg(pack.value(QStringLiteral("description")).toString())
                .arg(skillNames.join(QStringLiteral(", ")));
            QStringList reasons;
            for (const auto &reason : pack.value(QStringLiteral("reasons")).toArray()) {
                reasons << reason.toString();
            }
            if (!reasons.isEmpty()) {
                detail += QStringLiteral("\n  Detectado: ") + reasons.join(QStringLiteral("; "));
            }
            lines << detail;
        }
        return lines.join(QLatin1Char('\n'));
    }

    if (area == QStringLiteral("projects")) {
        const QJsonArray registered = payload.value(QStringLiteral("registered_projects")).toArray();
        lines << QStringLiteral("Proyectos registrados: %1").arg(registered.size());
        for (const auto &value : registered) {
            const auto item = value.toObject();
            const QString trust = item.value(QStringLiteral("trusted")).toBool()
                ? QStringLiteral("confiable") : QStringLiteral("requiere trust");
            lines << QStringLiteral("%1 · %2 · %3 · %4 · %5")
                .arg(item.value(QStringLiteral("alias")).toString(),
                     item.value(QStringLiteral("name")).toString(),
                     item.value(QStringLiteral("profile")).toString(),
                     item.value(QStringLiteral("status")).toString(), trust);
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("project")) {
        lines << QStringLiteral("Confianza: %1 · estado: %2")
            .arg(workspace.value(QStringLiteral("trusted")).toBool() ? QStringLiteral("confiable") : QStringLiteral("requiere trust"),
                 workspace.value(QStringLiteral("status")).toString());
        lines << QStringLiteral("Sesión: %1")
            .arg(workspace.value(QStringLiteral("session_name")).toString(QStringLiteral("sin sesión")));
        lines << QStringLiteral("Procesos configurados: %1 · agentes configurados: %2")
            .arg(QString::number(workspace.value(QStringLiteral("configured_process_count")).toInt()),
                 QString::number(workspace.value(QStringLiteral("configured_agent_count")).toInt()));
        for (const auto &warning : workspace.value(QStringLiteral("warnings")).toArray()) {
            lines << QStringLiteral("Aviso: %1").arg(warning.toString());
        }
        if (!workspace.value(QStringLiteral("error")).isNull()) {
            lines << QStringLiteral("Configuración: %1").arg(workspace.value(QStringLiteral("error")).toString());
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("agents")) {
        lines << QStringLiteral("Agentes configurados: %1").arg(workspace.value(QStringLiteral("configured_agent_count")).toInt());
        for (const auto &value : payload.value(QStringLiteral("agents")).toArray()) {
            const auto agent = value.toObject();
            lines << QStringLiteral("%1 · %2 · sesión %3 · pane %4")
                .arg(agent.value(QStringLiteral("id")).toString(),
                     agent.value(QStringLiteral("available")).toBool() ? QStringLiteral("disponible") : QStringLiteral("no disponible"),
                     agent.value(QStringLiteral("session")).toString(),
                     agent.value(QStringLiteral("pane")).toString());
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("processes")) {
        lines << QStringLiteral("Procesos configurados: %1").arg(workspace.value(QStringLiteral("configured_process_count")).toInt());
        for (const auto &value : payload.value(QStringLiteral("processes")).toArray()) {
            const auto process = value.toObject();
            QString detail = QStringLiteral("%1 · %2")
                .arg(process.value(QStringLiteral("label")).toString(), process.value(QStringLiteral("status")).toString());
            if (process.value(QStringLiteral("pid")).isDouble()) {
                detail += QStringLiteral(" · PID %1").arg(process.value(QStringLiteral("pid")).toInt());
            }
            if (process.value(QStringLiteral("port")).isDouble()) {
                detail += QStringLiteral(" · puerto %1").arg(process.value(QStringLiteral("port")).toInt());
            }
            lines << detail;
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("resources")) {
        const QJsonObject processResources = resources.value(QStringLiteral("processes")).toObject();
        const QJsonObject governor = resources.value(QStringLiteral("governor")).toObject();
        lines << QStringLiteral("RAM: %1 / %2 · %3")
            .arg(formatBytes(memory.value(QStringLiteral("used_bytes")).toDouble()),
                 formatBytes(memory.value(QStringLiteral("total_bytes")).toDouble()),
                 displayPercent(memory.value(QStringLiteral("percent"))));
        lines << QStringLiteral("Swap: %1 / %2 · %3")
            .arg(formatBytes(swap.value(QStringLiteral("used_bytes")).toDouble()),
                 formatBytes(swap.value(QStringLiteral("total_bytes")).toDouble()),
                 displayPercent(swap.value(QStringLiteral("percent"))));
        lines << QStringLiteral("CPU: %1 · procesos del proyecto: %2, %3")
            .arg(displayPercent(resources.value(QStringLiteral("cpu_percent"))),
                 QString::number(processResources.value(QStringLiteral("running_count")).toInt()),
                 formatBytes(processResources.value(QStringLiteral("rss_bytes")).toDouble()));
        lines << QStringLiteral("Governor: aviso %1% · crítico %2% · swap %3% · máximo %4 agentes")
            .arg(QString::number(governor.value(QStringLiteral("warning_memory_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("critical_memory_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("warning_swap_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("max_agents")).toInt()));
        return lines.join(QLatin1Char('\n'));
    }

    const QJsonArray stacks = project.value(QStringLiteral("stacks")).toArray();
    QStringList stackNames;
    for (const auto &stack : stacks) {
        stackNames << stack.toString();
    }
    lines << QStringLiteral("Stack: %1").arg(stackNames.isEmpty() ? QStringLiteral("sin detectar") : stackNames.join(QStringLiteral(", ")));
    lines << QStringLiteral("Confianza: %1 · Workspace: %2")
        .arg(workspace.value(QStringLiteral("trusted")).toBool() ? QStringLiteral("confiable") : QStringLiteral("requiere trust"),
             workspace.value(QStringLiteral("status")).toString());
    lines << QStringLiteral("RAM: %1 · Swap: %2 · CPU: %3")
        .arg(displayPercent(memory.value(QStringLiteral("percent"))),
             displayPercent(swap.value(QStringLiteral("percent"))),
             displayPercent(resources.value(QStringLiteral("cpu_percent"))));
    return lines.join(QLatin1Char('\n'));
}

QString MainWindow::formatResourcesOverview(const QJsonObject &payload) const
{
    const QJsonObject system = payload.value(QStringLiteral("system")).toObject();
    QStringList lines;
    lines << QStringLiteral("Sistema · RAM %1 / %2 (%3) · swap %4 / %5 (%6) · CPU %7")
        .arg(formatBytes(system.value(QStringLiteral("memory_used_bytes")).toDouble()),
             formatBytes(system.value(QStringLiteral("memory_total_bytes")).toDouble()),
             displayPercent(system.value(QStringLiteral("memory_percent"))),
             formatBytes(system.value(QStringLiteral("swap_used_bytes")).toDouble()),
             formatBytes(system.value(QStringLiteral("swap_total_bytes")).toDouble()),
             displayPercent(system.value(QStringLiteral("swap_percent"))),
             displayPercent(system.value(QStringLiteral("cpu_percent"))));
    lines << QStringLiteral("Consumo de procesos gestionados por proyecto:");
    for (const auto &value : payload.value(QStringLiteral("projects")).toArray()) {
        const auto project = value.toObject();
        const QString trust = project.value(QStringLiteral("trusted")).toBool()
            ? QStringLiteral("confiable") : QStringLiteral("sin trust");
        lines << QStringLiteral("%1 · %2 · %3 · %4 procesos · RAM %5 · CPU %6 · Governor %7/%8 · agentes %9 · %10")
            .arg(project.value(QStringLiteral("alias")).toString(),
                 project.value(QStringLiteral("name")).toString(),
                 project.value(QStringLiteral("status")).toString(),
                 QString::number(project.value(QStringLiteral("running_count")).toInt()),
                 formatBytes(project.value(QStringLiteral("rss_bytes")).toDouble()),
                 displayPercent(project.value(QStringLiteral("cpu_percent"))),
                 QString::number(project.value(QStringLiteral("warning_memory_percent")).toInt()),
                 QString::number(project.value(QStringLiteral("critical_memory_percent")).toInt()),
                 QString::number(project.value(QStringLiteral("max_agents")).toInt()), trust);
    }
    if (payload.value(QStringLiteral("projects")).toArray().isEmpty()) {
        lines << QStringLiteral("No hay proyectos registrados.");
    }
    return lines.join(QLatin1Char('\n'));
}
