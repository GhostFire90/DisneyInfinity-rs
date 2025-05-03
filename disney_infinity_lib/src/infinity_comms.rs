use std::{array, num::Wrapping, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, RwLock}, thread::{spawn, JoinHandle} };
use hidapi::{HidApi, HidDevice, HidError};
use crate::commands;
use crate::{InfnityResult, InfinityError};



pub(crate) type StoredFn<R> = Box<dyn FnMut() -> R + Send>;


pub(crate) struct InfinityComms{
    device : Arc<Mutex<HidDevice>>,
    message_number : Wrapping<u8>,
    pending_messages : Arc<[RwLock<MessageFuture>; 256]>,
    finished : Arc<AtomicBool>,
    com_thread : Option<JoinHandle<()>>,
    observers : Arc<Mutex<Vec<StoredFn<()>>>>
}
#[derive(Default, Debug)]
struct MessageFuture{
    data : Option<Vec<u8>>
}
impl MessageFuture{
    pub(self) fn set_data(&mut self, data : Vec<u8>) {
        self.data = Some(data);
    }
    pub(self) fn get_state(&self) -> Option<Vec<u8>>{
        self.data.clone()
    }
}






fn com_thread(dev : Arc<Mutex<HidDevice>>, finished : Arc<AtomicBool>, messages : Arc<[RwLock<MessageFuture>; 256]>, observers : Arc<Mutex<Vec<StoredFn<()>>>>){
    while !finished.load(Ordering::Relaxed){
        let mut buf = [0u8; 32];
        if let Ok(dev_guard) = dev.lock(){
            if let Ok(_i) = dev_guard.read(&mut buf){
                let (command, length, mid) = (buf[0], buf[1], buf[2]);
                drop(dev_guard);
                match command{
                    commands::RESPONSE => {
                        let mut msg : Vec<u8> = buf[3usize..(length+2) as usize].to_vec();
                
                        msg.push(command);
                        match messages[mid as usize].write(){
                            Ok(mut guard) => guard.set_data(msg),
                            Err(e) => panic!("Message {} RWLock Poisoned, did a thread crash? Value : {:?}", mid, *e.into_inner() ),
                        }
                        
                    },
                    commands::NOTIFY_OBSERVERS => {
                        match observers.lock(){
                            Ok(mut obs_guard) => obs_guard.iter_mut().for_each(|x|x()),
                            Err(_) => panic!("Observer mutex poisoned, did a thread crash?"),
                        }

                        
                    },
                    _ => {

                        //sleep(Duration::from_millis(TIMEOUT_MS as u64));
                    }
                }
                
            }
        } 
        
        
    }
    
    
}



impl InfinityComms{
    pub fn close(&mut self){
        self.finished.store(true, Ordering::Relaxed);
        self.com_thread.take().unwrap().join().unwrap();
    }
    pub fn new() -> InfnityResult<Self>{
        let hid_context = HidApi::new()?;
        let device = Arc::new(Mutex::new(hid_context.open(0x0e6f, 0x0129)?)); 
        device.lock().unwrap().set_blocking_mode(false)?;
        let pending_messages = Arc::new(array::from_fn(|_|Default::default()));
        let finished = Arc::new(AtomicBool::new(false));
        let observers = Arc::new(Mutex::new(Vec::new()));
        let (dc, fc, pc, obs) = (device.clone(), finished.clone(), pending_messages.clone(), observers.clone());

        let cm_thread = spawn(||{
            com_thread(dc, fc, pc, obs);
        });

        Ok(Self 
        {
            device,
            message_number: Wrapping(0),
            pending_messages,
            finished,
            com_thread : Some(cm_thread),
            observers
        })
    }
    fn next_message_num(&mut self) -> u8{
        self.message_number += 1;
        self.message_number.0
    }
    fn construct_message(&mut self, command : u8, data : &[u8]) -> Result<(u8, [u8; 33]), HidError>{
        let mut message = [0u8;33];
        let id = self.next_message_num();
        let command_body : Vec<u8> = [command, id].iter().chain(data).cloned().collect();
        let command_length = (command_body.len() % 256) as u8;
        let mut checksum = Wrapping(0u8);
        let full_command : Vec<u8> = [0x0, 0xff, command_length].iter().chain(command_body.iter()).cloned().collect();
        full_command.iter().enumerate().for_each(|(i,x)|{
            message[i] = *x;
            checksum += *x;
        });
        message[full_command.len()] = checksum.0;
        Ok((id, message))
    }
    pub fn send_message(&mut self, command : u8, data : &[u8]) -> InfnityResult<u8>{
        let (id, message) = self.construct_message(command, data)?;

        if let Ok(dev_guard) = self.device.lock(){
            dev_guard.write(&message)?;
            let mut guard = self.pending_messages[id as usize].write().unwrap();
            *guard = Default::default();
            Ok(id)
        }
        else{
            Err(InfinityError::TexPoison)
        }     
        
    }
    pub fn recieve_message(&mut self, id : u8) -> Result<(u8,Vec<u8>), HidError>{
        loop{
            let guard = self.pending_messages[id as usize].read().unwrap();
            match guard.get_state(){
                Some(mut x) => return Ok((x.pop().unwrap(), x)),
                None => {
                    drop(guard);
                    //sleep(Duration::from_millis(TIMEOUT_MS as u64));
                },
            }
        }        
    }
    pub fn add_observer(&mut self, observer : Box<dyn FnMut() + Send>) -> InfnityResult<()>{
        if let Ok(mut guard) = self.observers.lock(){
            guard.push(observer);
            Ok(())
        } 
        else{
            Err(InfinityError::TexPoison)
        }
    }
    
}


