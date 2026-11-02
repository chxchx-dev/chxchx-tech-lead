#include "agent_chat_settings.hpp"
#include "bridge_schemas.hpp"
#include "chat_pill_picker.hpp"

#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>

namespace {

QString defaultLabel(const QString &value)
{
    return value.isEmpty() ? QStringLiteral("Por defecto") : QStringLiteral("%1 · por defecto").arg(value);
}

} // namespace

AgentChatSettings::AgentChatSettings(QString projectPath, QString agentId, QString cliProgram, QWidget *parent)
    : QWidget(parent)
    , m_projectPath(std::move(projectPath))
    , m_agentId(std::move(agentId))
    , m_cliProgram(std::move(cliProgram))
{
    setObjectName(QStringLiteral("chatSettings"));
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(6);
    m_model = new ChatPillPicker(this);
    m_model->setToolTip(QStringLiteral("Modelo para los próximos mensajes de este chat."));
    m_effort = new ChatPillPicker(this);
    m_effort->setToolTip(QStringLiteral("Nivel de razonamiento: más alto piensa más, tarda más y gasta más."));
    m_model->addItem(defaultLabel({}), QString());
    m_effort->addItem(defaultLabel({}), QString());
    layout->addWidget(m_model);
    layout->addWidget(m_effort);
    connect(m_model, &ChatPillPicker::currentIndexChanged, this, &AgentChatSettings::refreshEfforts);

    m_process.setWorkingDirectory(m_projectPath);
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this, [this] {
        const auto document = QJsonDocument::fromJson(m_process.readAllStandardOutput());
        if (document.isObject()) applyOptions(document.object());
    });
    load();
}

AgentChatSettings::~AgentChatSettings()
{
    if (m_process.state() == QProcess::NotRunning) return;
    m_process.kill();
    m_process.waitForFinished(1000);
}

QString AgentChatSettings::model() const
{
    return m_model->currentData().toString();
}

QString AgentChatSettings::effort() const
{
    return m_effort->currentData().toString();
}

void AgentChatSettings::load()
{
    if (m_cliProgram.isEmpty()) return;
    m_process.start(m_cliProgram, {QStringLiteral("bridge"), QStringLiteral("agent-options"), m_projectPath,
        QStringLiteral("--agent"), m_agentId});
}

void AgentChatSettings::applyOptions(const QJsonObject &payload)
{
    if (payload.value(QStringLiteral("schema")).toString() == BridgeSchemas::Error) {
        emit availabilityChanged(false, payload.value(QStringLiteral("error")).toString());
        return;
    }
    if (payload.value(QStringLiteral("schema")).toString() != BridgeSchemas::AgentOptions) return;
    m_models = payload.value(QStringLiteral("models")).toArray();
    m_defaultEffort = payload.value(QStringLiteral("default_effort")).toString();
    m_model->blockSignals(true);
    m_model->clear();
    m_model->addItem(defaultLabel(payload.value(QStringLiteral("default_model")).toString()), QString());
    for (const auto &value : m_models) {
        const QJsonObject model = value.toObject();
        m_model->addItem(model.value(QStringLiteral("label")).toString(), model.value(QStringLiteral("id")).toString());
    }
    m_model->blockSignals(false);
    refreshEfforts();
    emit providerChanged(payload.value(QStringLiteral("provider")).toString(),
        payload.value(QStringLiteral("default_model")).toString());
    emit availabilityChanged(payload.value(QStringLiteral("available")).toBool(),
        payload.value(QStringLiteral("install_hint")).toString());
}

void AgentChatSettings::refreshEfforts()
{
    // Each model accepts its own levels; keep the choice when it still applies.
    const QString previous = effort();
    const QString modelId = model();
    QJsonArray efforts;
    QString modelDefault = m_defaultEffort;
    for (const auto &value : m_models) {
        const QJsonObject entry = value.toObject();
        if (modelId.isEmpty() || entry.value(QStringLiteral("id")).toString() == modelId) {
            for (const auto &level : entry.value(QStringLiteral("efforts")).toArray()) {
                if (!efforts.contains(level)) efforts.append(level);
            }
            // A configured effort wins over the model's own default.
            if (!modelId.isEmpty() && m_defaultEffort.isEmpty())
                modelDefault = entry.value(QStringLiteral("default_effort")).toString();
            if (!modelId.isEmpty()) break;
        }
    }
    m_effort->clear();
    m_effort->addItem(defaultLabel(modelDefault), QString());
    for (const auto &level : efforts) m_effort->addItem(level.toString(), level.toString());
    const int index = m_effort->findData(previous);
    m_effort->setCurrentIndex(index >= 0 ? index : 0);
}
