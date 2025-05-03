
use crate::infinity_comms::StoredFn;
use crate::{commands, infinity_comms::InfinityComms};
use crate::InfnityResult;




/// Disney Infinity Base API
pub struct InfinityBase{
    comms : InfinityComms,
    
}



impl InfinityBase{
    /// Creates a new InfinityBase instance
    pub fn new() -> InfnityResult<Self>{
        let mut comms = InfinityComms::new()?;
        comms.send_message(commands::ACTIVATE, crate::ACTIVATE_MESSAGE.as_bytes())?;
        Ok(InfinityBase { comms })
    }
    /// Sets the color on the given platform  
    /// # Returns  
    /// returns Ok(()) if there was no issue with the underlying hid device  
    /// # Parameters   
    /// - platform: 1 indexed platform starting from the top at 1 and going counter-clockwise  
    /// - color: color represented as `[r, g, b]` in byte values  
    pub fn set_color(&mut self, platform : u8, color : [u8;3]) -> InfnityResult<()>{

        self.comms.send_message(commands::SOLID_COLOR, &[&[platform][..], &color[..]].concat())?;
        Ok(())
    }
    
    // [(platform, index)]
    fn parse_indexes(bytes : Vec<u8>) -> Vec<(u8, u8)>{
        bytes.iter().filter(|x| **x != 0x09).map(|b| ((*b & 0xf0) >> 4,  *b & 0x0f)).collect()        
    }

    pub(self) fn get_tag_idx(&mut self) -> InfnityResult<Vec<(u8, u8)>>{
        let id = self.comms.send_message(commands::INDEXES, &[])?;
        let (_, data) = self.comms.recieve_message(id)?;
        Ok(Self::parse_indexes(data))           
    }
    pub(self) fn get_tag(&mut self, idx : u8) -> InfnityResult<Vec<u8>>{
        let msg_id = self.comms.send_message(commands::GET_TAG, &[idx])?;
        Ok(self.comms.recieve_message(msg_id)?.1)
    }

    /// Gets all the identifiers of figured on the base  
    /// # Returns  
    /// Ok if no errors with underlying hid device    
    /// the inner array is structured as so    
    /// `tags[platform-1] == None` means no tag on platform    
    /// `tags[platform-1] == Some(val)` means there is a tag on the platform and the value is its unique ID    
    pub fn get_all_tags(&mut self) -> InfnityResult<[Option<[u8;8]>;3]>{
        let indexes = self.get_tag_idx()?;
        let mut result = [None;3];
        for (platform, indexes) in indexes{
            let id = self.get_tag(indexes)?;
            let mut tag = [0;8];
            tag.copy_from_slice(&id[0..8]);
            result[platform as usize-1] = Some(tag)
        }

        Ok(result)
    } 
    /// Adds a function to the list of functions to be called when the tags on the base update
    /// # Examples
    /// ```
    /// let mut base = InfinityBase::new().unwrap(); 
    /// let trigger = Arc::new(AtomicBool::new(false)); // trigger to be set on tags changed
    /// 
    /// let tc = trigger.clone(); // clone to be moved into the closure
    /// base.add_observer(Box::new(move || tc.store(true, std::sync::atomic::Ordering::Relaxed))).unwrap(); // closure of the subscribed function
    /// 
    /// loop {
    ///     // Check if the trigger has been set
    ///     if trigger.load(std::sync::atomic::Ordering::Relaxed){
    ///         let tags = base.get_all_tags().unwrap(); // get all the triggers
    ///         for (i, x) in tags.iter().enumerate(){
    ///             match x {
    ///                 Some(tag) => {
    ///                     // set the platform to light up
    ///                     base.set_color( i as u8+1, [126;3]).unwrap();
    ///                     // print the id of the figure x on platform i+1 
    ///                     print!("Platform {} : [", i+1);
    ///                     tag.iter().for_each(|x| print!("{x:#x} "));
    ///                     println!("]");
    ///                 },
    ///                 None => {
    ///                     // turn off the platform led
    ///                     base.set_color(i as u8 + 1, [0;3]).unwrap();
    ///                 },
    ///             }
    ///         }
    ///         //reset trigger
    ///         trigger.store(false, std::sync::atomic::Ordering::Relaxed);
    ///     }
    /// }
    /// ```
    pub fn add_observer(&mut self, observer : StoredFn<()>) -> InfnityResult<()>{
        self.comms.add_observer(observer)
    }

}

impl Drop for InfinityBase{
    fn drop(&mut self) {
        for i in 1..=3{
            self.set_color(i, [0;3]).unwrap();
        }
        self.comms.close();
    }
}

