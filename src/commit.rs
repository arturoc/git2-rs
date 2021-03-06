use std::iter::Range;
use std::marker;
use std::str;
use libc;

use {raw, signature, Oid, Error, Signature, Tree, Time, Object};
use util::Binding;

/// A structure to represent a git [commit][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Commit<'repo> {
    raw: *mut raw::git_commit,
    _marker: marker::PhantomData<Object<'repo>>,
}

/// An iterator over the parent commits of a commit.
pub struct Parents<'commit, 'repo: 'commit> {
    range: Range<usize>,
    commit: &'commit Commit<'repo>,
}

/// An iterator over the parent commits' ids of a commit.
pub struct ParentIds<'commit> {
    range: Range<usize>,
    commit: &'commit Commit<'commit>,
}

impl<'repo> Commit<'repo> {
    /// Get the id (SHA1) of a repository commit
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_commit_id(&*self.raw)) }
    }

    /// Get the id of the tree pointed to by this commit.
    ///
    /// No attempts are made to fetch an object from the ODB.
    pub fn tree_id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_commit_tree_id(&*self.raw)) }
    }

    /// Get the tree pointed to by a commit.
    pub fn tree(&self) -> Result<Tree<'repo>, Error> {
        let mut ret = 0 as *mut raw::git_tree;
        unsafe {
            try_call!(raw::git_commit_tree(&mut ret, &*self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_commit { self.raw }

    /// Get the full message of a commit.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message(&self) -> Option<&str> {
        str::from_utf8(self.message_bytes()).ok()
    }

    /// Get the full message of a commit as a byte slice.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    pub fn message_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_message(&*self.raw)).unwrap()
        }
    }

    /// Get the encoding for the message of a commit, as a string representing a
    /// standard encoding name.
    ///
    /// `None` will be returned if the encoding is not known
    pub fn message_encoding(&self) -> Option<&str> {
        let bytes = unsafe {
            ::opt_bytes(self, raw::git_commit_message(&*self.raw))
        };
        bytes.map(|b| str::from_utf8(b).unwrap())
    }

    /// Get the full raw message of a commit.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message_raw(&self) -> Option<&str> {
        str::from_utf8(self.message_raw_bytes()).ok()
    }

    /// Get the full raw message of a commit.
    pub fn message_raw_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_message_raw(&*self.raw)).unwrap()
        }
    }

    /// Get the full raw text of the commit header.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn raw_header(&self) -> Option<&str> {
        str::from_utf8(self.raw_header_bytes()).ok()
    }

    /// Get the full raw text of the commit header.
    pub fn raw_header_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_raw_header(&*self.raw)).unwrap()
        }
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs or if the summary is not valid
    /// utf-8.
    pub fn summary(&mut self) -> Option<&str> {
        self.summary_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs
    pub fn summary_bytes(&mut self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_commit_summary(self.raw)) }
    }

    /// Get the commit time (i.e. committer time) of a commit.
    ///
    /// The first element of the tuple is the time, in seconds, since the epoch.
    /// The second element is the offset, in minutes, of the time zone of the
    /// committer's preferred time zone.
    pub fn time(&self) -> Time {
        unsafe {
            Time::new(raw::git_commit_time(&*self.raw) as i64,
                      raw::git_commit_time_offset(&*self.raw) as i32)
        }
    }

    /// Creates a new iterator over the parents of this commit.
    pub fn parents<'a>(&'a self) -> Parents<'a, 'repo> {
        let max = unsafe { raw::git_commit_parentcount(&*self.raw) as usize };
        Parents {
            range: range(0, max),
            commit: self,
        }
    }

    /// Creates a new iterator over the parents of this commit.
    pub fn parent_ids(&self) -> ParentIds {
        let max = unsafe { raw::git_commit_parentcount(&*self.raw) as usize };
        ParentIds {
            range: range(0, max),
            commit: self,
        }
    }

    /// Get the author of this commit.
    pub fn author(&self) -> Signature {
        unsafe {
            let ptr = raw::git_commit_author(&*self.raw);
            signature::from_raw_const(self, ptr)
        }
    }

    /// Get the committer of this commit.
    pub fn committer(&self) -> Signature {
        unsafe {
            let ptr = raw::git_commit_committer(&*self.raw);
            signature::from_raw_const(self, ptr)
        }
    }

    /// Amend this existing commit with all non-`None` values
    ///
    /// This creates a new commit that is exactly the same as the old commit,
    /// except that any non-`None` values will be updated. The new commit has
    /// the same parents as the old commit.
    ///
    /// For information about `update_ref`, see `new`.
    pub fn amend(&self,
                 update_ref: Option<&str>,
                 author: Option<&Signature>,
                 committer: Option<&Signature>,
                 message_encoding: Option<&str>,
                 message: Option<&str>,
                 tree: Option<&Tree<'repo>>) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        let update_ref = try!(::opt_cstr(update_ref));
        let encoding = try!(::opt_cstr(message_encoding));
        let message = try!(::opt_cstr(message));
        unsafe {
            try_call!(raw::git_commit_amend(&mut raw,
                                            self.raw(),
                                            update_ref,
                                            author.map(|s| s.raw()),
                                            committer.map(|s| s.raw()),
                                            encoding,
                                            message,
                                            tree.map(|t| t.raw())));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Get the specified parent of the commit.
    ///
    /// Use the `parents` iterator to return an iterator over all parents.
    pub fn parent(&self, i: usize) -> Result<Commit<'repo>, Error> {
        unsafe {
            let mut raw = 0 as *mut raw::git_commit;
            try_call!(raw::git_commit_parent(&mut raw, &*self.raw,
                                             i as libc::c_uint));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the specified parent id of the commit.
    ///
    /// This is different from `parent`, which will attemptstempt to load the
    /// parent commit from the ODB.
    ///
    /// Use the `parent_ids` iterator to return an iterator over all parents.
    pub fn parent_id(&self, i: usize) -> Result<Oid, Error> {
        unsafe {
            let id = raw::git_commit_parent_id(self.raw, i as libc::c_uint);
            if id.is_null() {
                Err(Error::from_str("parent index out of bounds"))
            } else {
                Ok(Binding::from_raw(id))
            }
        }
    }
}

impl<'repo> Binding for Commit<'repo> {
    type Raw = *mut raw::git_commit;
    unsafe fn from_raw(raw: *mut raw::git_commit) -> Commit<'repo> {
        Commit {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_commit { self.raw }
}


impl<'repo, 'commit> Iterator for Parents<'commit, 'repo> {
    type Item = Commit<'repo>;
    fn next(&mut self) -> Option<Commit<'repo>> {
        self.range.next().map(|i| self.commit.parent(i).unwrap())
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}

impl<'repo, 'commit> DoubleEndedIterator for Parents<'commit, 'repo> {
    fn next_back(&mut self) -> Option<Commit<'repo>> {
        self.range.next_back().map(|i| self.commit.parent(i).unwrap())
    }
}

impl<'repo, 'commit> ExactSizeIterator for Parents<'commit, 'repo> {}

impl<'commit> Iterator for ParentIds<'commit> {
    type Item = Oid;
    fn next(&mut self) -> Option<Oid> {
        self.range.next().map(|i| self.commit.parent_id(i).unwrap())
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}

impl<'commit> DoubleEndedIterator for ParentIds<'commit> {
    fn next_back(&mut self) -> Option<Oid> {
        self.range.next_back().map(|i| self.commit.parent_id(i).unwrap())
    }
}

impl<'commit> ExactSizeIterator for ParentIds<'commit> {}

#[unsafe_destructor]
impl<'repo> Drop for Commit<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_commit_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let mut commit = repo.find_commit(target).unwrap();
        assert_eq!(commit.message(), Some("initial"));
        assert_eq!(commit.id(), target);
        commit.message_raw().unwrap();
        commit.raw_header().unwrap();
        commit.message_encoding();
        commit.summary().unwrap();
        commit.tree_id();
        commit.tree().unwrap();
        assert_eq!(commit.parents().count(), 0);

        assert_eq!(commit.author().name(), Some("name"));
        assert_eq!(commit.author().email(), Some("email"));
        assert_eq!(commit.committer().name(), Some("name"));
        assert_eq!(commit.committer().email(), Some("email"));

        let sig = repo.signature().unwrap();
        let tree = repo.find_tree(commit.tree_id()).unwrap();
        let id = repo.commit(Some("HEAD"), &sig, &sig, "bar", &tree,
                             &[&commit]).unwrap();
        let head = repo.find_commit(id).unwrap();

        let new_head = head.amend(Some("HEAD"), None, None, None,
                                  Some("new message"), None).unwrap();
        let new_head = repo.find_commit(new_head).unwrap();
        assert_eq!(new_head.message(), Some("new message"));

        repo.find_object(target, None).unwrap().as_commit().unwrap();
    }
}

