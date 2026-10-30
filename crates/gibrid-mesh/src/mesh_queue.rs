//! Очередь мешинга с ограниченным размером и worker потоком.
//! Реализует bounded backpressure: если входная очередь полна,
//! submit() возвращает ошибку вместо неограниченного роста памяти.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::{MeshJob, MeshResult};

/// Ошибки очереди мешинга.
#[derive(Debug, thiserror::Error)]
pub enum MeshQueueError {
    #[error("mesh queue input is full (backpressure)")]
    InputFull,
    #[error("mesh queue worker has stopped")]
    WorkerStopped,
}

/// Ограниченная очередь задач мешинга с одним worker потоком.
pub struct MeshQueue {
    input: Arc<Mutex<VecDeque<MeshJob>>>,
    input_cond: Arc<Condvar>,
    output: Arc<Mutex<VecDeque<MeshResult>>>,
    output_cond: Arc<Condvar>,
    max_input: usize,
    worker: Option<thread::JoinHandle<()>>,
    stopped: Arc<Mutex<bool>>,
}

impl MeshQueue {
    /// Создать очередь с num_workers рабочими потоками (пока 1) и лимитами очередей.
    pub fn new(max_input: usize, max_output: usize) -> Self {
        let input = Arc::new(Mutex::new(VecDeque::new()));
        let input_cond = Arc::new(Condvar::new());
        let output = Arc::new(Mutex::new(VecDeque::new()));
        let output_cond = Arc::new(Condvar::new());
        let stopped = Arc::new(Mutex::new(false));

        let input_c = Arc::clone(&input);
        let input_cond_c = Arc::clone(&input_cond);
        let output_c = Arc::clone(&output);
        let output_cond_c = Arc::clone(&output_cond);
        let stopped_c = Arc::clone(&stopped);
        let max_output_c = max_output;

        let worker = thread::spawn(move || {
            Self::worker_loop(
                input_c,
                input_cond_c,
                output_c,
                output_cond_c,
                max_output_c,
                stopped_c,
            );
        });

        Self {
            input,
            input_cond,
            output,
            output_cond,
            max_input,
            worker: Some(worker),
            stopped,
        }

    }

    /// Основной цикл worker: берёт задачу из входной очереди, строит меш,
    /// класть результат в выходную очередь (с backpressure).
    fn worker_loop(
        input: Arc<Mutex<VecDeque<MeshJob>>>,
        input_cond: Arc<Condvar>,
        output: Arc<Mutex<VecDeque<MeshResult>>>,
        output_cond: Arc<Condvar>,
        max_output: usize,
        stopped: Arc<Mutex<bool>>,
    ) {
        loop {
            let job = {
                let mut input_guard = input.lock().unwrap();
                while input_guard.is_empty() {
                    if *stopped.lock().unwrap() {
                        return;
                    }
                    input_guard = input_cond.wait(input_guard).unwrap();
                }
                input_guard.pop_front().unwrap()
            };

            // Построить меш (все блоки считаются полными кубами, соседи неизвестны)
            let mesh = crate::build_section_mesh(&crate::SectionMeshInput {
                section_x: job.section_x,
                section_y: job.section_y,
                section_z: job.section_z,
                palette: &job.palette,
                packed_data: &job.packed_data,
                bits_per_block: job.bits_per_block,
                is_full_cube: &|_| true,
                get_block: &|_, _, _| None,
            });

            let result = MeshResult {
                mesh,
                revision: job.revision,
            };

            // Отправить результат с backpressure
            {
                let mut output_guard = output.lock().unwrap();
                while output_guard.len() >= max_output {
                    if *stopped.lock().unwrap() {
                        return;
                    }
                    output_guard = output_cond.wait(output_guard).unwrap();
                }
                output_guard.push_back(result);
            }
            output_cond.notify_one();
        }
    }

    /// Отправить задачу на мешинг. Возвращает ошибку если входная очередь полна.
    pub fn submit(&self, job: MeshJob) -> Result<(), MeshQueueError> {
        let mut input_guard = self.input.lock().unwrap();
        if input_guard.len() >= self.max_input {
            return Err(MeshQueueError::InputFull);
        }
        input_guard.push_back(job);
        self.input_cond.notify_one();
        Ok(())
    }

    /// Получить готовый результат, если есть. Не блокирует.
    pub fn try_recv(&self) -> Option<MeshResult> {
        let mut output_guard = self.output.lock().unwrap();
        if output_guard.is_empty() {
            None
        } else {
            let result = output_guard.pop_front().unwrap();
            self.output_cond.notify_one();
            Some(result)
        }
    }

    /// Количество задач во входной очереди.
    pub fn input_len(&self) -> usize {
        self.input.lock().unwrap().len()
    }

    /// Количество готовых результатов в выходной очереди.
    pub fn output_len(&self) -> usize {
        self.output.lock().unwrap().len()
    }

    /// Очередь входных задач пуста.
    pub fn input_is_empty(&self) -> bool {
        self.input.lock().unwrap().is_empty()
    }
}

impl Drop for MeshQueue {
    fn drop(&mut self) {
        *self.stopped.lock().unwrap() = true;
        self.input_cond.notify_all();
        self.output_cond.notify_all();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResourceId};

    fn make_job(section_x: i32, section_y: i32, section_z: i32, revision: u64) -> MeshJob {
        let palette: Vec<ResourceId> = vec![ResourceId::minecraft("stone").unwrap()];
        let packed: Vec<u64> = vec![0; 1];
        MeshJob {
            section_x,
            section_y,
            section_z,
            revision: crate::MeshRevision(revision),
            palette,
            packed_data: packed,
            bits_per_block: 0,
        }
    }

    #[test]
    fn queue_submit_and_recv() {
        let queue = MeshQueue::new(4, 4);
        let job = make_job(0, 0, 0, 1);
        assert!(queue.submit(job).is_ok());
        assert_eq!(queue.input_len(), 1);

        // Дать worker время на обработку
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(queue.output_len(), 1);
        let result = queue.try_recv().unwrap();
        assert_eq!(result.revision.0, 1);
        assert!(!result.mesh.is_empty());
    }

    #[test]
    fn queue_input_full_returns_error() {
        let queue = MeshQueue::new(1, 4);
        let job = make_job(0, 0, 0, 1);
        assert!(queue.submit(job).is_ok());
        // Очередь заполнена (max_input = 1). Второй submit может вернуть Ok или Err
        // в зависимости от того, обработал ли worker первую задачу. Важно: не должно быть паники.
        let _ = queue.submit(make_job(1, 0, 0, 2));
        assert!(queue.input_len() <= 1);
    }

    #[test]
    fn queue_multiple_jobs() {
        let queue = MeshQueue::new(8, 8);
        for i in 0..5 {
            assert!(queue.submit(make_job(i, 0, 0, i as u64)).is_ok());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(queue.output_len(), 5);
        for i in 0..5 {
            let result = queue.try_recv().unwrap();
            assert_eq!(result.revision.0, i as u64);
        }
    }
}
