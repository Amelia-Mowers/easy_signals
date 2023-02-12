use std::rc::Rc;
use std::cell::RefCell;
use core::slice::Iter;
use std::collections::VecDeque;
use easy_events::Event;

pub trait SignalObserver{
    fn process_signal(&mut self, event: Rc<dyn Event>);
}

pub trait SignalSubject {
    fn get_observers_iter(&self) -> Iter<Rc<RefCell<dyn SignalObserver>>>;
    fn subscribe_observer(&mut self, new_observer: Rc<RefCell<dyn SignalObserver>>);
    fn copy_observers(&self) -> Vec<Rc<RefCell<dyn SignalObserver>>> {
        let mut copy = Vec::new();
        for o in self.get_observers_iter() {
            copy.push(o.clone());
        }
        copy
    }
    fn send_signal(&self, event: Rc<dyn Event>) {
        for o in self.get_observers_iter() {
            o.borrow_mut().process_signal(event.clone());
        }
    }
    fn get_signal_snapshot(&self, event: Rc<dyn Event>) -> SignalSnapShot {
        SignalSnapShot {
            event,
            subs: self.copy_observers()
        }
    }
    fn send_signal_to(&self, event: Rc<dyn Event>, targets: &Vec<Rc<RefCell<dyn SignalObserver>>>) {
        for o in self.get_observers_iter() {
            o.borrow_mut().process_signal(event.clone());
        }
        for o in targets.iter() {
            o.borrow_mut().process_signal(event.clone());
        }
    }
    fn get_signal_to_snapshot(&self, event: Rc<dyn Event>, targets: &Vec<Rc<RefCell<dyn SignalObserver>>>) -> SignalSnapShot {
        let mut subs = self.copy_observers();
        subs.append(&mut targets.clone());
        SignalSnapShot {
            event,
            subs
        }
    }
}
  
#[macro_export]
macro_rules! implement_signal_subject {
    (
        $struct:ident, 
        $observers_field:ident
    ) => {
        impl SignalSubject for $struct {
            fn get_observers_iter(&self) -> Iter<Rc<RefCell<dyn SignalObserver>>> {
                self.$observers_field.iter()
            }

            fn subscribe_observer(&mut self, new_observer: Rc<RefCell<dyn SignalObserver>>) {
                self.$observers_field.retain(|x| !Rc::ptr_eq(x, &new_observer));
                self.$observers_field.push(new_observer);
            }
        }
    }
}

pub struct SignalSnapShot {
    event:  Rc<dyn Event>,
    subs: Vec<Rc<RefCell<dyn SignalObserver>>>
}

implement_signal_subject!(SignalSnapShot, subs);

impl SignalSnapShot {
    pub fn execute(&self) {
        self.send_signal(self.event.clone());
    }
}

pub struct SignalQueue {
    signal_queue: RefCell<VecDeque<SignalSnapShot>>
}

impl SignalQueue {
    pub fn new() -> Self {
        Self {
            signal_queue: RefCell::new(VecDeque::new())
        }
    }

    pub fn push(&self, signal: SignalSnapShot) {
        self.signal_queue.borrow_mut().push_back(signal);
    }

    fn pop(&self) -> Option<SignalSnapShot> {
        self.signal_queue.borrow_mut().pop_front()
    }

    pub fn next_signal(&self) -> Option<SignalSnapShot> {
        if let Some(s) = self.pop() {
            s.execute();
            Some(s)
        } else {
            None
        }
    }
    
    fn is_empty(&self) -> bool {
        self.signal_queue.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::rc::Weak;
    use easy_self_referencing_objects::{SelfReferencing, implement_self_referencing};
    use easy_events::implement_event;
    
    struct Observer {
        me: Weak<RefCell<Self>>
    }

    impl Observer {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new_cyclic(|me| {
                RefCell::new(Self { me: me.clone() })
            })
        }
    }

    implement_self_referencing!(Observer, me);

    impl SignalObserver for Observer {
        fn process_signal(&mut self, event: Rc<dyn Event>) {
            if let Some(_e) = event.as_any().downcast_ref::<EventA>() {
                println!("Observer received EventA");
            }

            if let Some(_e) = event.as_any().downcast_ref::<EventB>() {
                println!("Observer received EventB");
            }

            if let Some(_e) = event.as_any().downcast_ref::<EventC>() {
                println!("Observer received EventC");
            }
        }
    }

    struct ObserverBackground {
        me: Weak<RefCell<Self>>
    }

    impl ObserverBackground {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new_cyclic(|me| {
                RefCell::new(Self { me: me.clone() })
            })
        }
    }

    implement_self_referencing!(ObserverBackground, me);

    impl SignalObserver for ObserverBackground {
        fn process_signal(&mut self, event: Rc<dyn Event>) {
            if let Some(_e) = event.as_any().downcast_ref::<EventA>() {
                println!("Background Observer received EventA");
            }

            if let Some(_e) = event.as_any().downcast_ref::<EventB>() {
                println!("Background Observer received EventB");
            }

            if let Some(_e) = event.as_any().downcast_ref::<EventC>() {
                println!("Background Observer received EventC");
            }
        }
    }
    
    struct Subject {
        me: Weak<RefCell<Self>>,
        subs: Vec<Rc<RefCell<dyn SignalObserver>>>
    }

    impl Subject {
        fn new() -> Rc<RefCell<Self>> {
            Rc::new_cyclic(|me| {
                RefCell::new(Self { 
                    me: me.clone(),
                    subs: Vec::new()
                 })
            })
        }
    }

    implement_self_referencing!(Subject, me);
    implement_signal_subject!(Subject, subs);

    struct EventA;
    implement_event!(EventA);

    struct EventB;
    implement_event!(EventB);

    struct EventC;
    implement_event!(EventC);

    #[test]
    fn add_and_signal_observer() {
        let subject = Subject::new();
        let observer = Observer::new();

        subject.borrow_mut().subscribe_observer(observer.clone());
        subject.borrow().send_signal(Rc::new(EventA{}));
        subject.borrow().send_signal(Rc::new(EventB{}));
    }

    #[test]
    fn queue_test() {
        let queue = SignalQueue::new();
        let subject = Subject::new();
        let observer = Observer::new();

        subject.borrow_mut().subscribe_observer(observer.clone());

        queue.push(subject.borrow().get_signal_snapshot(Rc::new(EventA{})));
        queue.next_signal();

        queue.push(subject.borrow().get_signal_snapshot(Rc::new(EventB{})));
        queue.push(subject.borrow().get_signal_snapshot(Rc::new(EventA{})));
        queue.next_signal();
        queue.push(subject.borrow().get_signal_snapshot(Rc::new(EventB{})));
        queue.next_signal();
        queue.next_signal();
    }

    #[test]
    fn targeted_signal_test() {
        let queue = SignalQueue::new();
        let subject = Subject::new();
        let observer_target = Observer::new();
        let observer_background = ObserverBackground::new();

        subject.borrow_mut().subscribe_observer(observer_background.clone());

        subject.borrow().send_signal(Rc::new(EventA{}));
        
        subject.borrow().send_signal_to(Rc::new(EventB{}), &vec![observer_target.clone()]);

        queue.push(subject.borrow().get_signal_to_snapshot(Rc::new(EventC{}), &vec![observer_target.clone()]));
        queue.next_signal();
    }
}
