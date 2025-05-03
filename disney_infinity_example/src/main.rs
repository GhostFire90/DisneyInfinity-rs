use std::pin::Pin;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

const ASYNC : bool = true;
use disney_infinity_lib::InfinityBase;
use disney_infinity_lib::AsyncInfinityBase;
use tokio::sync::Mutex;


#[tokio::main]
async fn main(){
    if ASYNC{
        asy_main().await;
    }
    else{
        threaded_main();
    }
}

async fn color_updator(bc : Arc<Mutex<AsyncInfinityBase>>){
    // lock the mutex on the device
    let mut guard = bc.lock().await;
    // read the tags
    let tags = guard.get_all_tags().await.unwrap();
    for (i, x) in tags.iter().enumerate(){
        match x {
            Some(tag) => {
                // Set color of platform i+1 to white
                guard.set_color( i as u8+1, [126;3]).await.unwrap();
                // Log the id of figure x on platform i+1
                print!("Platform {} : [", i+1);
                tag.iter().for_each(|x| print!("{x:#x} "));
                println!("]");
            },
            None => {
                // Set the platform i+1 to black, turning it off
                guard.set_color(i as u8 + 1, [0;3]).await.unwrap();
            },
        }
    }
}

async fn asy_main(){
    let base = Arc::new(Mutex::new(AsyncInfinityBase::new().await.unwrap()));

    //clone arc for usage in the outer closure
    let bc = base.clone();
    let outer = move || {
        // clone it again every call for usage in the inner function
        let bc = bc.clone();
        // pin the future of the async function
        Box::pin(color_updator(bc)) as Pin<Box<dyn Future<Output = ()> + Send>>
    };

    // lock and add the observer
    base.lock().await.add_observer(Box::new(outer)).unwrap();
    loop {
        
        std::thread::sleep(Duration::from_secs(1));
    }


}

fn threaded_main() { 
    
    let mut base = InfinityBase::new().unwrap(); 
    let trigger = Arc::new(AtomicBool::new(false)); // trigger to be set on tags changed

    let tc = trigger.clone(); // clone to be moved into the closure
    base.add_observer(Box::new(move || tc.store(true, std::sync::atomic::Ordering::Relaxed))).unwrap(); // closure of the subscribed function
    


    loop {
        // Check if the trigger has been set
        if trigger.load(std::sync::atomic::Ordering::Relaxed){
            let tags = base.get_all_tags().unwrap(); // get all the triggers
            for (i, x) in tags.iter().enumerate(){
                match x {
                    Some(tag) => {
                        // set the platform to light up
                        base.set_color( i as u8+1, [126;3]).unwrap();
                        // print the id of the figure x on platform i+1 
                        print!("Platform {} : [", i+1);
                        tag.iter().for_each(|x| print!("{x:#x} "));
                        println!("]");
                    },
                    None => {
                        // turn off the platform led
                        base.set_color(i as u8 + 1, [0;3]).unwrap();
                    },
                }
            }
            //reset trigger
            trigger.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }


}
