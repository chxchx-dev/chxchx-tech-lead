#include "main_window.hpp"
#include "integrations/bridge_client.hpp"

#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QPlainTextEdit>
#include <QStatusBar>

void MainWindow::showHandoff(const QJsonObject &payload)
{
    if (!payload.value(QStringLiteral("exists")).toBool()) {
        m_handoffPreview->setPlainText(QStringLiteral("Todavía no existe .ai/HANDOFF.md en este proyecto."));
        return;
    }
    m_handoffSummary->setText(payload.value(QStringLiteral("summary")).toString());
    m_handoffPending->setText(payload.value(QStringLiteral("pending")).toString());
    m_handoffValidation->setText(payload.value(QStringLiteral("validation")).toString());
    m_handoffPreview->setPlainText(payload.value(QStringLiteral("content")).toString());
}

void MainWindow::showMemory(const QJsonObject &payload)
{
    m_memoryList->clear();
    const QJsonArray notes = payload.value(QStringLiteral("notes")).toArray();
    for (const auto &value : notes) {
        const auto note = value.toObject();
        const QString title = note.value(QStringLiteral("title")).toString();
        const QString modified = note.value(QStringLiteral("modified_at")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2").arg(title, modified), m_memoryList);
        item->setData(Qt::UserRole, note.value(QStringLiteral("content")).toString());
        item->setToolTip(note.value(QStringLiteral("path")).toString());
    }
    const QString query = payload.value(QStringLiteral("query")).toString();
    if (notes.isEmpty()) {
        m_memorySummary->setText(query.isEmpty()
            ? QStringLiteral("No hay notas locales en .ai/memory.")
            : QStringLiteral("No hay notas que coincidan con la búsqueda."));
        m_memoryDetail->clear();
    } else {
        m_memorySummary->setText(QStringLiteral("%1 nota(s) · solo lectura%2")
            .arg(QString::number(notes.size()), query.isEmpty()
                ? QString() : QStringLiteral(" · filtro: %1").arg(query)));
        m_memoryList->setCurrentRow(0);
    }
}

void MainWindow::selectMemoryNote(QListWidgetItem *item)
{
    m_memoryDetail->setPlainText(item == nullptr
        ? QStringLiteral("Selecciona una nota para leerla.")
        : item->data(Qt::UserRole).toString());
}

void MainWindow::showConversations(const QJsonObject &payload)
{
    m_chatList->clear();
    const QJsonArray conversations = payload.value(QStringLiteral("conversations")).toArray();
    for (const auto &value : conversations) {
        const auto conversation = value.toObject();
        const QString provider = conversation.value(QStringLiteral("provider")).toString();
        const QString title = conversation.value(QStringLiteral("title")).toString();
        const QString modified = conversation.value(QStringLiteral("modified_at")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2 · %3").arg(provider, modified, title), m_chatList);
        item->setData(Qt::UserRole, provider);
        item->setData(Qt::UserRole + 1, conversation.value(QStringLiteral("session_id")).toString());
        item->setData(Qt::UserRole + 2, conversation.value(QStringLiteral("preview")).toString());
        item->setToolTip(conversation.value(QStringLiteral("transcript_path")).toString());
    }
    const QString query = payload.value(QStringLiteral("query")).toString();
    m_chatSummary->setText(conversations.isEmpty()
        ? (query.isEmpty()
            ? QStringLiteral("No hay conversaciones locales para este proyecto.")
            : QStringLiteral("No hay conversaciones que coincidan con la búsqueda."))
        : QStringLiteral("%1 conversaciones · Codex y Claude Code · solo lectura")
            .arg(conversations.size()));
    m_chatDetail->clear();
    if (!conversations.isEmpty()) {
        m_chatDetail->setPlainText(QStringLiteral("Selecciona una conversación para leerla."));
    }
}

void MainWindow::showConversation(const QJsonObject &payload)
{
    QStringList lines;
    lines << QStringLiteral("%1 · %2 · %3 · solo lectura")
        .arg(payload.value(QStringLiteral("provider")).toString(),
             payload.value(QStringLiteral("title")).toString(),
             payload.value(QStringLiteral("modified_at")).toString());
    lines << payload.value(QStringLiteral("transcript_path")).toString() << QString();
    const QString provider = payload.value(QStringLiteral("provider")).toString();
    for (const auto &value : payload.value(QStringLiteral("messages")).toArray()) {
        const auto message = value.toObject();
        const QString role = message.value(QStringLiteral("role")).toString();
        lines << QStringLiteral("%1:\n%2")
            .arg(role == QStringLiteral("user") ? QStringLiteral("Tú") : provider,
                 message.value(QStringLiteral("text")).toString());
    }
    m_chatDetail->setPlainText(lines.join(QStringLiteral("\n\n")));
}

void MainWindow::showErrors(const QJsonObject &payload)
{
    m_errorsList->clear();
    const QJsonArray errors = payload.value(QStringLiteral("errors")).toArray();
    for (const auto &value : errors) {
        const auto error = value.toObject();
        const QString date = error.value(QStringLiteral("occurred_at")).toString();
        const QString operation = error.value(QStringLiteral("operation")).toString();
        const QString message = error.value(QStringLiteral("message")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2 · %3")
            .arg(date.left(19).replace(QLatin1Char('T'), QLatin1Char(' ')), operation,
                 message.simplified().left(120)), m_errorsList);
        item->setData(Qt::UserRole, QStringLiteral("%1\nProyecto: %2 · %3\nFecha UTC: %4\n\n%5")
            .arg(operation,
                 error.value(QStringLiteral("project")).toString(),
                 error.value(QStringLiteral("project_path")).toString(),
                 date,
                 message));
    }
    m_errorsSummary->setText(errors.isEmpty()
        ? QStringLiteral("No hay errores guardados para este proyecto.")
        : QStringLiteral("%1 errores recientes · caché local %2")
            .arg(QString::number(errors.size()), payload.value(QStringLiteral("cache_path")).toString()));
    m_errorDetail->setPlainText(QStringLiteral("Selecciona un error para leer el detalle."));
}

void MainWindow::selectChatConversation(QListWidgetItem *item)
{
    if (item == nullptr) {
        return;
    }
    if (m_bridgeClient->isRunning()) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine la consulta actual."));
        return;
    }
    const QStringList arguments = {
        QStringLiteral("bridge"), QStringLiteral("conversation"), m_projectPath,
        item->data(Qt::UserRole).toString(), item->data(Qt::UserRole + 1).toString()};
    runCommand(arguments);
}
