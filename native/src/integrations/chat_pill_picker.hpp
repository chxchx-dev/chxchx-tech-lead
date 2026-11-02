#pragma once

#include <QToolButton>
#include <QVariant>

class QActionGroup;
class QMenu;

// Rounded pill that opens a menu of choices (like model pickers in IDE chats).
// Mirrors the small QComboBox API the chat needs.
class ChatPillPicker final : public QToolButton {
    Q_OBJECT
public:
    explicit ChatPillPicker(QWidget *parent = nullptr);

    void addItem(const QString &label, const QVariant &data);
    void clear();
    int count() const;
    int currentIndex() const { return m_current; }
    void setCurrentIndex(int index);
    int findData(const QVariant &data) const;
    QVariant currentData() const;
    QString currentText() const;

signals:
    void currentIndexChanged(int index);

private:
    void updateLabel();

    QMenu *m_menu = nullptr;
    QActionGroup *m_group = nullptr;
    int m_current = -1;
};
