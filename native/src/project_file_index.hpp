#pragma once

#include <QObject>
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
    void rebuild();

signals:
    void indexChanged(int fileCount, bool ready);

private:
    void acceptIndex(QStringList paths);

    QString m_projectRoot;
    QStringList m_paths;
    QPointer<QThread> m_thread;
    std::shared_ptr<std::atomic_bool> m_cancel;
    bool m_ready = false;
};
