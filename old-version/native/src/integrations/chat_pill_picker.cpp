#include "chat_pill_picker.hpp"

#include <QAction>
#include <QActionGroup>
#include <QMenu>

ChatPillPicker::ChatPillPicker(QWidget *parent)
    : QToolButton(parent)
{
    setObjectName(QStringLiteral("pillPicker"));
    setPopupMode(QToolButton::InstantPopup);
    setToolButtonStyle(Qt::ToolButtonTextOnly);
    setCursor(Qt::PointingHandCursor);
    // Allow shrinking below the label width; the composer switches to compact text.
    setMinimumWidth(56);
    m_menu = new QMenu(this);
    m_menu->setObjectName(QStringLiteral("pillMenu"));
    m_group = new QActionGroup(this);
    m_group->setExclusive(true);
    setMenu(m_menu);
    connect(m_group, &QActionGroup::triggered, this, [this](QAction *action) {
        setCurrentIndex(int(m_menu->actions().indexOf(action)));
    });
}

void ChatPillPicker::addItem(const QString &label, const QVariant &data)
{
    QAction *action = m_menu->addAction(label);
    action->setData(data);
    action->setCheckable(true);
    m_group->addAction(action);
    if (m_current < 0) setCurrentIndex(0);
}

void ChatPillPicker::clear()
{
    for (QAction *action : m_menu->actions()) {
        m_group->removeAction(action);
        delete action;
    }
    m_current = -1;
    updateLabel();
}

int ChatPillPicker::count() const
{
    return int(m_menu->actions().size());
}

void ChatPillPicker::setCurrentIndex(int index)
{
    const auto actions = m_menu->actions();
    if (index < 0 || index >= actions.size()) return;
    actions.at(index)->setChecked(true);
    const bool changed = index != m_current;
    m_current = index;
    updateLabel();
    if (changed && !signalsBlocked()) emit currentIndexChanged(index);
}

int ChatPillPicker::findData(const QVariant &data) const
{
    const auto actions = m_menu->actions();
    for (qsizetype index = 0; index < actions.size(); ++index) {
        if (actions.at(index)->data() == data) return int(index);
    }
    return -1;
}

QVariant ChatPillPicker::currentData() const
{
    const auto actions = m_menu->actions();
    return m_current >= 0 && m_current < actions.size() ? actions.at(m_current)->data() : QVariant();
}

QString ChatPillPicker::currentText() const
{
    const auto actions = m_menu->actions();
    return m_current >= 0 && m_current < actions.size() ? actions.at(m_current)->text() : QString();
}

void ChatPillPicker::setCompact(bool compact)
{
    if (compact == m_compact) return;
    m_compact = compact;
    updateLabel();
}

void ChatPillPicker::updateLabel()
{
    QString label = currentText();
    if (m_compact) label = label.section(QStringLiteral(" · "), 0, 0);
    setText(label.isEmpty() ? QStringLiteral("…  ▾") : label + QStringLiteral("  ▾"));
    setToolTip(currentText());
}
