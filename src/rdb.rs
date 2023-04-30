pub mod db {
	use std::{
		io,
		io::prelude::*,
		io::SeekFrom::*,
		io::ErrorKind,
		fs,
		fs::File,
		fs::OpenOptions,
		thread,
	};

	pub struct Db {
    	entno: u64,
    	fl: File,
		id: File,
    	fl_name: String,
	}

	pub type Entry = Vec<String>;

	impl Db {
    	pub fn open(filename: &str) -> Result<Self, ()> {
			let mut entno: u64 = 0;
			let fl_name: String;
			let fl = match OpenOptions::new().read(true)
				.write(true).open(filename) {
	    			Ok(mut ret) => {
						Self::lines_proc(&mut ret, &mut entno);
						fl_name = filename.to_string();
						ret
	    			}
	    			Err(error) => {
						match error.kind() {
		    				ErrorKind::NotFound => {
								// It's super improbable for this to Err,
								// but for the sake of completeness, we
								// handle that case as well
								match OpenOptions::new().read(true).write(true)
									.create(true).open(filename) {
			    						Ok(ret) => {
											fl_name = filename.to_string();
											ret
			    						}
			    						Err(_) => { return Err(()); }
									}
		    				}
		    				_ => { return Err(()); }
						}
	    			}
				};
			let id = match OpenOptions::new().read(true)
				.write(true).create(true).open("index") {
					Ok(ret) => ret,
					Err(_) => { panic!("Error creating index!"); }
				};
				Ok(Self {
	    			entno: entno,
	    			fl: fl,
					id: id,
	    			fl_name: fl_name })
    	}

		fn lines_proc(fl: &mut File, line_no: &mut u64) { // no threading yet
			let (mut i, mut b, mut ch): (u64, usize, [u8; 1]) = (0, 0, [0]);
			fl.seek(Start(0));
			let start_pos = fl.stream_position().unwrap();
			fl.seek(End(0));
			let end_pos = fl.stream_position().unwrap();
			if end_pos == start_pos { *line_no = 0; return(); }
			fl.seek(Current(-1));
			fl.read(&mut ch);
			if ch[0] != 10 { fl.write(&10_u8.to_le_bytes()); }
			fl.seek(Start(0));
			loop {
				b = fl.read(&mut ch).unwrap();
				if b == 0 { break; }
				else if ch[0] == 10 { i += 1; }
			}
			*line_no = i;
			return();
		}

		fn seek_line(&mut self, line_no: u64) {
			self.fl.seek(Start(0)); // file < line_no is not dealt with
			if line_no == 1 || line_no == 0 { return(); }
			self.id.seek(Start(8 * (line_no - 2)));
			let mut record_pos: [u8; 8] = [0; 8];
			match self.id.read(&mut record_pos) {
				Ok(ret) => { if ret != 8 { panic!("Error reading id!"); } }
				Err(_) => { panic!("Error reading id!"); }
			}
			let record_pos = u64::from_le_bytes(record_pos);
			self.fl.seek(Start(record_pos));
			return();
		}

    	pub fn append_entry(&mut self, ent: Entry) {
			self.fl.seek(End(0));
			for (i, prop) in ent.iter().enumerate() {
	    		self.fl.write(prop.as_bytes());
	    		if (i+1) != ent.len() { self.fl.write(&30_u8.to_le_bytes()); }
			}
			self.fl.write(&10_u8.to_le_bytes());
			self.entno += 1;
			return();
    	}

   		pub fn read_entry(&mut self, no: u64) -> Result<Entry, ()> {
			if self.entno < no || no == 0 { return Err(()); }
			self.seek_line(no);
			let mut ent: Entry = Vec::new();
			loop {
	    		let mut str_buff = String::new();
	    		let (mut ch, mut done): ([u8; 1], bool) = ([0], false);
	    		loop {
					ch[0] = 0;
					self.fl.read(&mut ch);
					if ch[0] == 30 { break; }
					else if ch[0] == 10 { done = true; break; }
					str_buff.push(ch[0] as char);
	    		}
	    		ent.push(str_buff);
	    		if done == true { break; }
			}
			Ok(ent)
		}

    	pub fn index(&mut self, t_no: u8) {
			let mut index_rec = OpenOptions::new().read(true).write(true)
	    		.create(true).open("id").unwrap(); // better have Db have a name
			let mut handle: thread::JoinHandle<_>;
			let mut handle_vec: Vec<thread::JoinHandle<_>> = Vec::new();
			for i in 0..t_no {
	    		let ct_no = i;
	    		let tt_no = t_no;
	    		let fl_name = self.fl_name.clone();
	    		handle = thread::spawn(move || {
					let mut cid = String::from("id");
					cid.push_str(&ct_no.to_string());
					let mut cid = OpenOptions::new().read(true)
		    			.write(true).create(true).open(cid).unwrap();
					let mut t_fl = OpenOptions::new().read(true)
		    			.open(fl_name).unwrap();
					t_fl.seek(End(0));
					let end_pos = t_fl.stream_position().unwrap();
					t_fl.seek(Start(ct_no as u64*(end_pos / tt_no as u64)));
					let end_pos = {
		    			if ct_no == tt_no - 1 { end_pos }
		    			else { (ct_no + 1) as u64*(end_pos / tt_no as u64) }
					};
					let mut curr_pos = t_fl.stream_position().unwrap();
					let mut ch: [u8; 1] = [0];
					loop {
		    			t_fl.read(&mut ch);
		    			curr_pos += 1;
		    			if ch[0] == 10 {
							cid.write(&curr_pos.to_le_bytes());
		    			}
		    			if curr_pos == end_pos { break; }
					}
	    		});
	    		handle_vec.push(handle);
			}
			for _ in 0..t_no {
	    		handle = handle_vec.pop().unwrap();
	    		handle.join();
			}
			for i in 0..t_no {
	    		let mut id_name = String::from("id");
	    		id_name.push_str(&i.to_string());
	    		let mut id = OpenOptions::new().read(true)
					.open(&id_name).unwrap();
	    		let mut record: [u8; 8] = [0; 8];
	    		let mut b: usize;
	    		loop {
					b = id.read(&mut record).unwrap();
					if b == 0 { break; }
					index_rec.write(&record);
	    		}
	    		drop(id);
	    		fs::remove_file(id_name);
			}
			return();
    	}

		pub fn close(self) -> io::Result<()> {
			let sync_status = self.fl.sync_all();
			drop(self);
			sync_status
		}
	}
}