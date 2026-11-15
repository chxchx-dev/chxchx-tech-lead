// Rendering of a `chxchx.local-models` snapshot: hardware, Ollama banner, table and actions.
#include "local_models_widget.hpp"

#include "local_models_widget_style.hpp"

#include <QCheckBox>
#include <QFrame>
#include <QLabel>
#include <QPushButton>
#include <QTreeWidget>

void LocalModelsWidget::applyPayload(const QJsonObject &payload)
{
    const QString previousTag = selectedRow() != nullptr ? selectedRow()->tag : QString();
    m_snapshot = parseLocalModelsPayload(payload);
    m_table->clear();
    if (!m_snapshot.error.isEmpty()) {
        m_hardware->setText(QStringLiteral("No pude leer el equipo: %1").arg(m_snapshot.error));
        updateBanner();
        updateDetail();
        updateActions();
        return;
    }
    m_hardware->setText(QStringLiteral("%1 de RAM (%2 libres) · %3 núcleos · %4 · %5 de disco libres")
        .arg(formatBytes(m_snapshot.totalRamBytes), formatBytes(m_snapshot.availableRamBytes))
        .arg(m_snapshot.cpuCores).arg(m_snapshot.accelerator, formatBytes(m_snapshot.diskFreeBytes)));
    if (!m_snapshot.loaded.isEmpty()) {
        QStringList names;
        for (const LoadedModelInfo &model : m_snapshot.loaded) {
            names << QStringLiteral("%1 (%2%3)").arg(model.name, formatBytes(model.sizeBytes),
                model.vramBytes > 0 ? QStringLiteral(", %1 en GPU").arg(formatBytes(model.vramBytes)) : QString());
        }
        m_hardware->setText(m_hardware->text() + QStringLiteral("\nEn memoria ahora: %1").arg(names.join(QStringLiteral(", "))));
    }
    QTreeWidgetItem *toSelect = nullptr;
    for (const LocalModelRow &row : m_snapshot.models) {
        auto *item = new QTreeWidgetItem(m_table);
        item->setText(0, row.recommended ? QStringLiteral("★ %1").arg(row.label) : row.label);
        item->setData(0, Qt::UserRole, row.tag);
        item->setText(1, formatBytes(row.sizeBytes));
        item->setText(2, fitLabel(row.fit));
        item->setForeground(2, fitColor(row.fit));
        item->setText(3, stateText(row));
        item->setToolTip(0, QStringLiteral("%1\n%2").arg(row.tag, row.note));
        if (row.tag == previousTag || (toSelect == nullptr && row.recommended && previousTag.isEmpty())) toSelect = item;
    }
    if (toSelect == nullptr && m_table->topLevelItemCount() > 0) toSelect = m_table->topLevelItem(0);
    if (toSelect != nullptr) m_table->setCurrentItem(toSelect);
    updateBanner();
    updateDetail();
    updateActions();
}

void LocalModelsWidget::updateBanner()
{
    QString message;
    bool installOffer = false;
    if (!m_snapshot.error.isEmpty()) {
        message.clear();
    } else if (!m_snapshot.ollamaInstalled) {
        installOffer = true;
        message = QStringLiteral("Ollama no está instalado. Studio no lo instala: usa el método oficial%1.")
            .arg(m_snapshot.installCommand.isEmpty() ? QStringLiteral(" (página de descarga)")
                                                     : QStringLiteral(":\n%1").arg(m_snapshot.installCommand));
    } else if (!m_snapshot.ollamaRunning) {
        message = QStringLiteral("Ollama está instalado pero no responde en %1. Inicia la aplicación o ejecuta `ollama serve` y pulsa Actualizar.")
            .arg(m_snapshot.ollamaUrl);
    } else if (!m_snapshot.opencodeInstalled) {
        message = QStringLiteral("OpenCode no está instalado: %1. Puedes preparar el modelo ahora; la configuración queda en opencode.json.")
            .arg(m_snapshot.opencodeHint);
    }
    m_banner->setVisible(!message.isEmpty());
    m_bannerText->setText(message);
    m_copyInstall->setVisible(installOffer && !m_snapshot.installCommand.isEmpty());
    m_openDownload->setVisible(installOffer && !m_snapshot.installUrl.isEmpty());
}

void LocalModelsWidget::updateDetail()
{
    const LocalModelRow *row = selectedRow();
    if (row == nullptr) {
        m_detail->clear();
        return;
    }
    QString text = QStringLiteral("<b>%1</b> (%2) — %3<br>%4<br>Memoria estimada con %5K de contexto: ~%6.")
        .arg(row->label.toHtmlEscaped(), row->tag.toHtmlEscaped(), row->note.toHtmlEscaped(),
             row->summary.toHtmlEscaped()).arg(m_snapshot.contextTokens / 1024).arg(formatBytes(row->requiredBytes));
    if (!row->downloaded && !row->diskOk) text += QStringLiteral("<br><b>No hay espacio en disco suficiente para descargarlo.</b>");
    m_detail->setText(text);
}

void LocalModelsWidget::updateActions()
{
    const LocalModelRow *row = selectedRow();
    const bool busy = isBusy();
    // A preview only reads, so it works with Ollama stopped; a real run needs a live server.
    const bool selectable = row != nullptr && m_snapshot.error.isEmpty() && !busy;
    const bool blockedByDisk = row != nullptr && !row->downloaded && !row->diskOk;
    m_preview->setEnabled(selectable && m_snapshot.ollamaInstalled);
    m_apply->setEnabled(selectable && m_snapshot.canSetUp() && !blockedByDisk);
    m_remove->setEnabled(selectable && m_snapshot.canSetUp() && (row->downloaded || row->configured));
    m_cancel->setVisible(busy);
    m_cancel->setEnabled(busy);
    m_reload->setEnabled(!busy);
    m_setDefault->setEnabled(!busy);
    m_table->setEnabled(!busy);
}
