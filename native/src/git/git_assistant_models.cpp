#include "git_assistant_models.hpp"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>

namespace {

const GitAssistantModel *find(const GitModelList &list, const QString &id)
{
    for (const GitAssistantModel &model : list.models) if (model.id == id) return &model;
    return nullptr;
}

QString effortFor(const GitModelList &list, const QString &modelId)
{
    // Haiku has no reasoning levels to lower; asking for one could make the request fail.
    if (modelId.contains(QLatin1String("haiku"), Qt::CaseInsensitive)) return {};
    const GitAssistantModel *model = find(list, modelId);
    return model != nullptr ? lowEffort(model->efforts) : QString();
}

} // namespace

GitModelList parseAgentOptions(const QByteArray &json)
{
    GitModelList list;
    const QJsonObject object = QJsonDocument::fromJson(json).object();
    if (object.value(QStringLiteral("schema")).toString() != QLatin1String("chxchx.agent-options")) return list;
    list.loaded = true;
    list.defaultModel = object.value(QStringLiteral("default_model")).toString();
    for (const QJsonValue &value : object.value(QStringLiteral("models")).toArray()) {
        const QJsonObject item = value.toObject();
        GitAssistantModel model;
        model.id = item.value(QStringLiteral("id")).toString();
        for (const QJsonValue &effort : item.value(QStringLiteral("efforts")).toArray()) model.efforts << effort.toString();
        if (!model.id.isEmpty()) list.models.append(model);
    }
    return list;
}

bool looksLight(const QString &modelId)
{
    static const QRegularExpression named(QStringLiteral("(mini|nano|haiku|flash|lite|small|tiny|instant)"), QRegularExpression::CaseInsensitiveOption);
    // Local models whose size says it: 0.5b … 14b (30b and 120b are not small).
    static const QRegularExpression sized(QStringLiteral("(?<![0-9.])(0\\.5|1\\.5|[1-9]|1[0-4])b(?![a-z0-9])"), QRegularExpression::CaseInsensitiveOption);
    return named.match(modelId).hasMatch() || sized.match(modelId).hasMatch();
}

QString lowEffort(const QStringList &efforts)
{
    if (efforts.contains(QLatin1String("low"))) return QStringLiteral("low");
    if (efforts.contains(QLatin1String("minimal"))) return QStringLiteral("minimal");
    return {};
}

GitModelChoice chooseModel(const GitModelList &list, const QString &preference)
{
    GitModelChoice choice;
    if (preference == QLatin1String(GitAgentDefaultModel)) {
        // The agent's own model, still with the lowest effort when that model is known to take one.
        choice.effort = effortFor(list, list.defaultModel);
        return choice;
    }
    if (!preference.isEmpty()) {
        choice.model = preference;
        choice.effort = effortFor(list, preference);
        return choice;
    }
    for (const GitAssistantModel &model : list.models) {
        if (!looksLight(model.id)) continue;
        choice.model = model.id;
        choice.effort = effortFor(list, model.id);
        choice.light = true;
        return choice;
    }
    choice.effort = effortFor(list, list.defaultModel);
    return choice;
}

QString describeChoice(const GitModelChoice &choice)
{
    QString text = choice.model.isEmpty() ? QStringLiteral("modelo del agente") : choice.model;
    if (!choice.effort.isEmpty()) text += QStringLiteral(" · esfuerzo %1").arg(choice.effort == QLatin1String("low") ? QStringLiteral("bajo") : choice.effort);
    return text;
}
