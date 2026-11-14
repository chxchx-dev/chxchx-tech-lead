#pragma once

#include <QDialog>

class QTextBrowser;

// A piece of advice from the assistant: readable Markdown, copyable, never executed.
class GitAdviceDialog final : public QDialog {
    Q_OBJECT
public:
    GitAdviceDialog(const QString &title, const QString &markdown, const QString &agentLabel, QWidget *parent = nullptr);
    QString markdown() const { return m_markdown; }
    QString renderedText() const;

private:
    QString m_markdown;
    QTextBrowser *m_view = nullptr;
};
