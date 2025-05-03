use std::{array, num::Wrapping, pin::Pin, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};

use tokio::{spawn, sync::{oneshot, Mutex as TokMutex}, task::JoinHandle};


use hidapi::{HidApi, HidError, HidDevice};

use crate::{commands, InfinityError, InfnityResult};

pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

type MessageChannel = oneshot::Sender<Vec<u8>>;
pub(crate) type StoredAsyncFn = Box<dyn Fn() -> BoxFuture + Send>;


pub(crate) struct AsyncComms{
    device : Arc<TokMutex<HidDevice>>,
    message_number : Wrapping<u8>,
    pending_messages : Arc<[Mutex<Option<MessageChannel>>;256]>,
    finished : Arc<AtomicBool>,
    com_task : Option<JoinHandle<()>>,
    observers : Arc<Mutex<Vec<StoredAsyncFn>>>
}

async fn com_task(dev : Arc<TokMutex<HidDevice>>, finished : Arc<AtomicBool>, messages : Arc<[Mutex<Option<MessageChannel>>;256]>, observers : Arc<Mutex<Vec<StoredAsyncFn>>>){
    while !finished.load(Ordering::Relaxed) {
        let dev_guard = dev.lock().await;
        let mut buf = [0u8;32];
        if let Ok(_i) = dev_guard.read(&mut buf){
            let (command, length, mid) = (buf[0], buf[1], buf[2]);
            drop(dev_guard);
            match command{
                commands::RESPONSE => {
                    let msg : Vec<u8> = buf[3..(length as usize)+2].to_vec();
                    match messages[mid as usize].lock(){
                        Ok(mut ms_guard) => {let _ = ms_guard.take().unwrap().send(msg);}, // discard because we dont care if someone is listening SCREAM TO THE VOID 
                        Err(e) => panic!("Message {} mutex poisoned, did a thread crash? Value : {:?}", mid, *e.into_inner()),
                    }
                    
                },
                commands::NOTIFY_OBSERVERS => {
                    match observers.lock(){
                        Ok(obs_guard) => {
                            for x in obs_guard.iter(){
                                spawn(x());                            
                            }
                        } ,
                        Err(_) => panic!("Observer mutex poisoned, did a thread crash?"),
                    }
                }
                _ => {}
                
            }
        }
    }
}

impl AsyncComms{
    pub async fn close(&mut self){
        self.finished.store(true, Ordering::Relaxed);
        self.com_task.take().unwrap().await.unwrap();
    }

    pub async fn new() -> InfnityResult<Self>{
        let hid_context = HidApi::new()?;
        let device = Arc::new(
            TokMutex::new(
                hid_context.open(0x0e6f, 0x0129)?
            )
        );
        
        device.lock().await.set_blocking_mode(false)?;
        let pending_messages = Arc::new(array::from_fn(|_| Mutex::new(None)));
        let finished = Arc::new(AtomicBool::new(false));
        let observers  = Arc::new(Mutex::new(Vec::new()));
        let com_task  =spawn(com_task(device.clone(), finished.clone(), pending_messages.clone(), observers.clone()));
        Ok(Self { device, message_number: Wrapping(0), pending_messages, finished, com_task: Some(com_task), observers })
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

    pub async fn send_message(&mut self, command : u8, data : &[u8]) -> InfnityResult<oneshot::Receiver<Vec<u8>>>{
        let (id, message) = self.construct_message(command, data)?;

        let (tx, rx) = oneshot::channel();
        let dev_guard= self.device.lock().await;
        dev_guard.write(&message)?;
        let mut guard = self.pending_messages[id as usize].lock().unwrap();
        *guard = Some(tx);
        Ok(rx)            
    }
    pub fn add_observer(&mut self, observer : StoredAsyncFn) -> InfnityResult<()>{
        if let Ok(mut guard) = self.observers.lock(){
            guard.push(observer);
            Ok(())
        } 
        else{
            Err(InfinityError::TexPoison)
        }
    }
    
}
