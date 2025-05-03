use crate::{async_comms::{AsyncComms, StoredAsyncFn}, commands, InfnityResult, ACTIVATE_MESSAGE};

/// Disney Infinity base API with async capabilities
pub struct AsyncInfinityBase{
    comms : AsyncComms
}


impl AsyncInfinityBase{
    /// Creates a new instance of the AsyncInfinityBase  
    pub async fn new() -> InfnityResult<Self>{
        let mut comms = AsyncComms::new().await?;
        comms.send_message(commands::ACTIVATE, ACTIVATE_MESSAGE.as_bytes()).await?;
        Ok(Self { comms })
    }
    /// Sets the color on the given platform  
    /// # Returns  
    /// returns Ok(()) if there was no issue with the underlying hid device  
    /// # Parameters   
    /// - platform: 1 indexed platform starting from the top at 1 and going counter-clockwise  
    /// - color: color represented as `[r, g, b]` in byte values  
    pub async fn set_color(&mut self, platform : u8, color : [u8;3]) -> InfnityResult<()>{
        self.comms.send_message(commands::SOLID_COLOR, &[&[platform][..], &color[..]].concat()).await?;
        Ok(())
    }
    fn parse_indexes(bytes : Vec<u8>) -> Vec<(u8, u8)>{
        bytes.iter().filter(|x| **x != 0x09).map(|b| ((*b & 0xf0) >> 4,  *b & 0x0f)).collect()        
    }
    async fn get_tag_idx(&mut self) -> InfnityResult<Vec<(u8, u8)>>{
        let id = self.comms.send_message(commands::INDEXES, &[]).await?;
        let data = id.await.unwrap();
        Ok(Self::parse_indexes(data))           
    }
    async fn get_tag(&mut self, idx : u8) -> InfnityResult<Vec<u8>>{
        let msg_id = self.comms.send_message(commands::GET_TAG, &[idx]).await?;
        Ok(msg_id.await.unwrap())
    }
    /// Gets all the identifiers of figured on the base  
    /// # Returns  
    /// Ok if no errors with underlying hid device    
    /// the inner array is structured as so    
    /// `tags[platform-1] == None` means no tag on platform    
    /// `tags[platform-1] == Some(val)` means there is a tag on the platform and the value is its unique ID    
    pub async fn get_all_tags(&mut self) -> InfnityResult<[Option<[u8;8]>;3]>{
        let indexes = self.get_tag_idx().await?;
        let mut result = [None;3];
        for (platform, indexes) in indexes{
            let id = self.get_tag(indexes).await?;
            let mut tag = [0;8];
            tag.copy_from_slice(&id[0..8]);
            result[platform as usize-1] = Some(tag)
        }

        Ok(result)
    }
    /// Adds a function to the list of functions to be spawned as tasks when the tags on the base update
    /// # Limitations 
    /// Due to the current limitations of rust and async closures, async callbacks are a lot harder to store, so there is a few more hoops to jump through
    /// make sure to pin the futures of the thing that creates it
    /// see examples for an idea of what I mean
    /// # Examples
    /// ```
    /// use std::pin::Pin;
    /// use std::sync::{atomic::AtomicBool, Arc};
    /// use tokio::sync::Mutex;
    /// use disney_infinity::AsyncInfinityBase;
    /// async fn color_updator(bc : Arc<Mutex<AsyncInfinityBase>>){
    ///     // lock the mutex on the device
    ///     let mut guard = bc.lock().await;
    ///     // read the tags
    ///     let tags = guard.get_all_tags().await.unwrap();
    ///     for (i, x) in tags.iter().enumerate(){
    ///         match x {
    ///             Some(tag) => {
    ///                 // Set color of platform i+1 to white
    ///                 guard.set_color( i as u8+1, [126;3]).await.unwrap();
    ///                 // Log the id of figure x on platform i+1
    ///                 print!("Platform {} : [", i+1);
    ///                 tag.iter().for_each(|x| print!("{x:#x} "));
    ///                 println!("]");
    ///             },
    ///             None => {
    ///                 // Set the platform i+1 to black, turning it off
    ///                 guard.set_color(i as u8 + 1, [0;3]).await.unwrap();
    ///             },
    ///         }
    ///     }
    /// }
    /// 
    /// async fn use_case(){
    ///     let base = Arc::new(Mutex::new(AsyncInfinityBase::new().await.unwrap()));
    /// 
    ///     //clone arc for usage in the outer closure
    ///     let bc = base.clone();
    ///     let outer = move || {
    ///         // clone it again every call for usage in the inner function
    ///         let bc = bc.clone();
    ///         // pin the future of the async function
    ///         Box::pin(color_updator(bc)) as Pin<Box<dyn Future<Output = ()> + Send>>
    ///     };
    /// 
    ///     // lock and add the observer
    ///     base.lock().await.add_observer(Box::new(outer)).unwrap();
    /// }
    /// ```
    pub fn add_observer(&mut self, observer : StoredAsyncFn) -> InfnityResult<()>{
        self.comms.add_observer(observer)
    }
    /// Call to shutdown communication with the device
    pub async fn close(mut self){
        for i in 1..=3{
            let _ = self.set_color(i, [0;3]).await;
        }
        self.comms.close().await;
    }
}

