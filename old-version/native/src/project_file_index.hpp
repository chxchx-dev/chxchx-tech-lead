#pragma once

#include <QObject>
#include <QJsonArray>
#include <QStringList>
#include <QPointer>

#include <atomic>
#include <memory>

class QThread;

class ProjectFileIndex final : public QObject {
    Q_OBJECT

public:
    explicit ProjectFileIndex(QString projectRoot, QObject *parent = nullptr);
    ~ProjectFileIndex() override;

    bool isReady() const;
    int fileCount() const;
    QStringList search(const QString &query, int limit = 250) const;
    void searchContent(const QString &query, int limit = 100);
    void rebuild();

signals:
    void indexChanged(int fileCount, bool ready);
    void contentSearchFinished(QString query, QJsonArray matches, bool truncated);

private:
    void acceptIndex(QStringList paths);
    void cancelContentSearch();

    QString m_projectRoot;
    QStringList m_paths;
    QPointer<QThread> m_thread;
    QPointer<QThread> m_contentSearchThread;
    std::shared_ptr<std::atomic_bool> m_cancel;
    std::shared_ptr<std::atomic_bool> m_contentSearchCancel;
    bool m_ready = false;
};
