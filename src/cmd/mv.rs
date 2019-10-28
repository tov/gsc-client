use crate::prelude::*;
use crate::messages::FileMetaChange;

impl GscClient {
    pub fn mv(&self, src: &RemotePattern, part_dst: &RemoteDestination) -> Result<()> {
        self.try_warn(|| {
            let     src = self.fetch_one_matching_filename(src)?;
            let mut dst = HwQual {
                hw:   src.hw,
                name: src.name.as_str(),
            };

            let mut message = FileMetaChange::default();

            if let Some(hw) = part_dst.hw {
                if hw != dst.hw {
                    dst.hw     = hw;
                    message.hw = Some(hw);
                }
            }

            if part_dst.name != src.name && !part_dst.name.is_empty() {
                dst.name = &part_dst.name;
                message.name = Some(dst.name.to_owned());
            }

            if message.hw.is_none() && message.name.is_none() {
                v2!("Source and destination are identical.");
                return Ok(());
            }

            let policy = &mut self.config.get_overwrite_policy();
            if self.is_okay_to_write_remote(policy, &dst)? {
                message.overwrite = true;
            } else {
                return Ok(());
            }

            let uri      = format!("{}{}", self.config.get_endpoint(), src.uri);
            let request  = self.http.patch(&uri).json(&message);
            v2!("Moving remote file ‘{}’ to ‘{}’...", src, dst);
            self.send_request(request)?;

            Ok(())
        });

        Ok(())
    }
}
